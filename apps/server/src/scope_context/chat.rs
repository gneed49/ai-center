//! Question-specific SQL retrieval; compiler obligations remain exhaustive.
use super::{MAX_CONTEXT_BYTES, MAX_SCOPES, ScopeSnapshot, ScopeStamp, SourceCandidate};
use crate::{
    context::ContextCandidate,
    error::{AppError, AppResult},
};
use serde_json::json;
use sqlx::{Postgres, Transaction, types::Json};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct Row {
    source_id: i64,
    project_id: i64,
    project_public_id: Uuid,
    graph_version: i64,
    scope_kind: String,
    source_kind: String,
    mandatory: bool,
    excerpt: bool,
    candidate: Json<ContextCandidate>,
    total_sources: i64,
    mandatory_count: i64,
    external_total: i64,
    external_duplicates: i64,
}
fn obligations_error() -> AppError {
    AppError::Invalid("Les règles obligatoires de la société dépassent le budget du contexte. Réduisez ou consolidez ces règles avant de poursuivre ; aucune règle n'a été écartée silencieusement.".into())
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn load_for_query(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    question: &str,
) -> AppResult<ScopeSnapshot> {
    let mut scopes:Vec<ScopeStamp>=sqlx::query_as("select id as project_id,public_id as project_public_id,graph_version,scope_kind from app.projects where workspace_id=app.current_workspace_id() and status='active' and (id=$1 or scope_kind='company') order by id")
        .bind(project_id).fetch_all(&mut **tx).await?;
    if !scopes.iter().any(|p| p.project_id == project_id) {
        return Err(AppError::NotFound);
    }
    let question = query_terms(question);
    let sql = include_str!("chat.sql")
        .replace("/* OBSERVATION_ROWS */", include_str!("observations.sql"));
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .bind(project_id)
        .bind(question)
        .fetch_all(&mut **tx)
        .await?;
    let total = rows.first().map_or(0, |r| r.total_sources);
    let external_total = rows.first().map_or(0, |r| r.external_total);
    let external_duplicates = rows.first().map_or(0, |r| r.external_duplicates);
    if rows.first().is_some_and(|r| r.mandatory_count > 160) {
        return Err(obligations_error());
    }
    let mut sources = Vec::new();
    let mut excerpts = Vec::new();
    let mut bytes = 0;
    let mut knowledge = 0;
    let mut artifacts = 0;
    let mut external = 0;
    let mut external_bytes = 0;
    // Reserve space for source/scopes/provenance envelopes before admission.
    let budget = MAX_CONTEXT_BYTES - 24000;
    for mut row in rows {
        let observed = matches!(
            row.source_kind.as_str(),
            "tool_source_observation" | "publication_observation"
        );
        if observed {
            if external >= super::observations::MAX_SOURCES {
                continue;
            }
            super::observations::enrich(tx, &mut row.candidate.0, &row.source_kind).await?;
        }
        let new_scope = !scopes.iter().any(|p| p.project_id == row.project_id);
        let size = serde_json::to_vec(&row.candidate.0)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .len()
            + row
                .candidate
                .0
                .observation
                .as_ref()
                .map_or(0, |value| value.to_string().len())
            + 300;
        let kind_limit = if row.source_kind == "knowledge_entry_version" {
            knowledge >= 160
        } else if observed {
            external >= super::observations::MAX_SOURCES
                || external_bytes + size > super::observations::MAX_EXTERNAL_BYTES
        } else {
            artifacts >= 20
        };
        if row.mandatory && (row.excerpt || bytes + size > budget || kind_limit) {
            return Err(obligations_error());
        }
        if !row.mandatory
            && (bytes + size > budget || kind_limit || (new_scope && scopes.len() >= MAX_SCOPES))
        {
            continue;
        }
        bytes += size;
        if new_scope {
            scopes.push(ScopeStamp {
                project_id: row.project_id,
                project_public_id: row.project_public_id,
                graph_version: row.graph_version,
                scope_kind: row.scope_kind,
            });
        }
        if row.excerpt {
            excerpts.push(row.candidate.0.version_public_id);
        }
        let kind = if row.source_kind == "knowledge_entry_version" {
            knowledge += 1;
            "knowledge_entry_version"
        } else if row.source_kind == "artifact_document_version" {
            artifacts += 1;
            "artifact_document_version"
        } else if observed {
            external += 1;
            external_bytes += size;
            if row.source_kind == "tool_source_observation" {
                "tool_source_observation"
            } else {
                "publication_observation"
            }
        } else {
            return Err(AppError::Internal("Unknown context source kind".into()));
        };
        sources.push(SourceCandidate {
            source_id: row.source_id,
            source_project_id: row.project_id,
            source_project_public_id: row.project_public_id,
            source_kind: kind,
            candidate: row.candidate.0,
        });
    }
    let included = i64::try_from(sources.len()).unwrap_or(i64::MAX);
    let snapshot = ScopeSnapshot {
        sources,
        scopes,
        summaries: vec![],
        summaries_truncated: false,
        retrieval: json!({"mode":"question_ranked_sql","exhaustive":false,"eligible_sources":total,"included_sources":included,
        "omitted_sources":total-included,"external_eligible_objects":external_total,"external_duplicate_sources":external_duplicates,"external_omitted_sources":external_total-i64::try_from(external).unwrap_or(i64::MAX),"excerpt_version_ids":excerpts,"max_scopes":MAX_SCOPES,"max_knowledge":160,"max_artifacts":20,"max_external_sources":super::observations::MAX_SOURCES,"max_external_bytes":super::observations::MAX_EXTERNAL_BYTES,
        "coverage_notice":"Contexte sélectionné pour cette question. Les sources omises et extraits ne permettent aucune affirmation de couverture exhaustive."}),
    };
    if serde_json::to_vec(&snapshot.context("preview", "preview"))
        .map_err(|e| AppError::Internal(e.to_string()))?
        .len()
        > MAX_CONTEXT_BYTES
    {
        return Err(obligations_error());
    }
    Ok(snapshot)
}

fn query_terms(question: &str) -> String {
    let terms = question
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() >= 3)
        .take(32)
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        "aicenteremptyquery".into()
    } else {
        terms.join(" | ")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terms_keep_recall_without_allowing_tsquery_syntax_injection() {
        assert_eq!(
            query_terms("Quelle règle quartzorion ?"),
            "quelle | règle | quartzorion"
        );
        assert_eq!(query_terms("':* | ! foo & bar"), "foo | bar");
        assert_eq!(query_terms("?"), "aicenteremptyquery");
    }
}
