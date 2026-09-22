//! Company source projection for the existing leased Steward pipeline.
use super::{CurrentKnowledgeVersion, ValidatedAssessment};
use crate::{error::AppResult, scope_context::ScopeStamp};
use serde_json::{Value, json};
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Debug, FromRow)]
#[allow(clippy::struct_field_names)] // Names mirror the typed provenance receipt columns.
pub(super) struct Source {
    pub source_id: i64,
    pub source_project_id: i64,
    pub source_project_public_id: Uuid,
    pub graph_version: i64,
    pub scope_kind: String,
    pub source_kind: String,
    pub source_public_id: Uuid,
    pub parent_public_id: Uuid,
    pub node_key: String,
    pub entry_type: String,
    pub title: String,
    pub statement: String,
    pub insufficient: bool,
    pub provenance: Value,
}
impl Source {
    fn snapshot(&self) -> Value {
        json!({"source_kind":self.source_kind,"source_project_public_id":self.source_project_public_id,
            "source_version_public_id":self.source_public_id,"title":self.title,"statement":self.statement,
            "read_status":if self.insufficient {"insufficient"} else {"text_observed"},
            "code_read": self.source_kind == "github_code_file_observation"
                && self.provenance.get("status").and_then(Value::as_str) == Some("code_read")
                && self.provenance.get("commit_verified").and_then(Value::as_bool) == Some(true),
            "provenance":self.provenance})
    }
    fn version(&self) -> CurrentKnowledgeVersion {
        CurrentKnowledgeVersion {
            version_id: self.source_id,
            version_public_id: self.source_public_id,
            knowledge_public_id: self.parent_public_id,
            node_key: self.node_key.clone(),
            entry_type: self.entry_type.clone(),
            title: self.title.clone(),
            statement: self.statement.clone(),
            rationale: self.snapshot().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub sources: HashMap<Uuid, Source>,
    pub scopes: Vec<ScopeStamp>,
    pub truncated: bool,
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
    limit: u32,
) -> AppResult<Option<(Vec<CurrentKnowledgeVersion>, Snapshot)>> {
    let enabled:bool=sqlx::query_scalar("select exists(select 1 from app.projects where workspace_id=app.current_workspace_id() and scope_kind='company' and status='active')")
        .fetch_one(&mut **tx).await?;
    if !enabled {
        return Ok(None);
    }
    let mut sources: Vec<Source> = sqlx::query_as(include_str!("company_sources.sql"))
        .bind(project)
        .bind(i64::from(limit) + 1)
        .fetch_all(&mut **tx)
        .await?;
    let truncated = sources.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    sources.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let mut seen = HashSet::new();
    let scopes = sources
        .iter()
        .filter(|source| seen.insert(source.source_project_id))
        .map(|source| ScopeStamp {
            project_id: source.source_project_id,
            project_public_id: source.source_project_public_id,
            graph_version: source.graph_version,
            scope_kind: source.scope_kind.clone(),
        })
        .collect();
    let versions = sources.iter().map(Source::version).collect();
    Ok(Some((
        versions,
        Snapshot {
            sources: sources
                .into_iter()
                .map(|source| (source.source_public_id, source))
                .collect(),
            scopes,
            truncated,
        },
    )))
}

/// Metadata, missing text and stale observations can support a request for
/// inspection, never a fabricated compatibility/contradiction verdict.
pub(super) fn enforce_observed_evidence(
    snapshot: &Snapshot,
    assessments: &mut [ValidatedAssessment],
) {
    for assessment in assessments {
        let insufficient = [assessment.left_public_id, assessment.right_public_id]
            .iter()
            .any(|id| {
                snapshot
                    .sources
                    .get(id)
                    .is_none_or(|source| source.insufficient)
            });
        if insufficient {
            assessment.classification = "ambiguous".into();
            assessment.severity = None;
            assessment.confidence = 0.0;
            assessment.title = "Vérification nécessaire : contenu insuffisant".into();
            assessment.explanation="Une source fournit seulement des métadonnées, un extrait incomplet ou une observation périmée/indisponible. Impossible de conclure à une contradiction ou une conformité. Seuls les fichiers et lignes explicitement signalés comme lus dans les sources constituent une observation de code.".into();
        }
    }
}

pub(super) async fn persist_source(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
    assessment: i64,
    role: &str,
    source: &Source,
) -> AppResult<()> {
    let knowledge = (source.source_kind == "knowledge_entry_version").then_some(source.source_id);
    let artifact = (source.source_kind == "artifact_document_version").then_some(source.source_id);
    let external =
        (source.source_kind == "external_reference_observation").then_some(source.source_id);
    let publication = (source.source_kind == "publication_observation").then_some(source.source_id);
    let code = (source.source_kind == "github_code_file_observation").then_some(source.source_id);
    sqlx::query("insert into app.steward_scope_sources(workspace_id,project_id,assessment_id,source_project_id,source_role,source_kind,source_public_id,
        knowledge_version_id,artifact_version_id,external_observation_id,publication_observation_id,github_code_file_observation_id,source_snapshot)
        values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) on conflict(assessment_id,source_role) do nothing")
        .bind(project).bind(assessment).bind(source.source_project_id).bind(role).bind(&source.source_kind).bind(source.source_public_id)
        .bind(knowledge).bind(artifact).bind(external).bind(publication).bind(code).bind(source.snapshot()).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_or_missing_content_cannot_authorize_a_provider_claim() {
        let left = Uuid::new_v4();
        let right = Uuid::new_v4();
        for kind in [
            "external_reference_observation",
            "publication_observation",
            "artifact_document_version",
        ] {
            let source = Source {
                source_id: 1,
                source_project_id: 1,
                source_project_public_id: Uuid::new_v4(),
                graph_version: 0,
                scope_kind: "project".into(),
                source_kind: kind.into(),
                source_public_id: right,
                parent_public_id: Uuid::new_v4(),
                node_key: "[FICTIF] metadata".into(),
                entry_type: "technical_rule".into(),
                title: "[FICTIF] Code absent ou observation périmée".into(),
                statement: "PR annoncée terminée ; contenu code non lu".into(),
                insufficient: true,
                provenance: json!({}),
            };
            let mut sources = HashMap::new();
            sources.insert(right, source.clone());
            let mut complete = source;
            complete.source_public_id = left;
            complete.insufficient = false;
            sources.insert(left, complete);
            let snapshot = Snapshot {
                sources,
                scopes: vec![],
                truncated: false,
            };
            for verdict in ["contradiction", "compatible"] {
                let mut assessments = vec![ValidatedAssessment {
                    left_public_id: left,
                    right_public_id: right,
                    classification: verdict.into(),
                    severity: Some("blocking".into()),
                    confidence: 0.99,
                    title: "[FICTIF] Conclusion fournisseur trop forte".into(),
                    explanation: "Code conforme".into(),
                    fingerprint: "a".repeat(64),
                }];
                enforce_observed_evidence(&snapshot, &mut assessments);
                assert_eq!(assessments[0].classification, "ambiguous");
                assert!(assessments[0].confidence.abs() < f64::EPSILON);
                assert!(assessments[0].severity.is_none());
                assert!(
                    assessments[0]
                        .explanation
                        .contains("explicitement signalés comme lus")
                );
                assert_eq!(snapshot.sources[&right].snapshot()["code_read"], false);
            }
        }
    }
}
