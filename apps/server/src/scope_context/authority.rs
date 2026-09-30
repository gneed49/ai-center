//! Recheck the credential authority captured before model I/O, without rereading providers.
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ObservationAuthority {
    pub source_kind: String,
    pub source_public_id: Uuid,
    pub connection_public_id: Uuid,
    pub connection_revision: i64,
}

const ROWS: &str = "select 'tool_source_observation'::text as source_kind,o.public_id as source_public_id,
  o.project_id,c.public_id as connection_public_id,r.connection_revision::bigint as connection_revision,
  app.tool_source_observation_current(o.id) as eligible
 from app.tool_source_observations o join app.tool_source_references r on r.id=o.reference_id
 join app.work_tool_connections c on c.id=r.connection_id
 where o.workspace_id=app.current_workspace_id() and o.public_id=any($1)
 union all select 'publication_observation',o.public_id,o.project_id,o.authority_connection_public_id,
  o.authority_connection_revision::bigint,app.publication_observation_current(o.id)
 from app.publication_source_observations o where o.workspace_id=app.current_workspace_id() and o.public_id=any($1)";

#[derive(sqlx::FromRow)]
struct Row {
    source_kind: String,
    source_public_id: Uuid,
    project_id: i64,
    connection_public_id: Option<Uuid>,
    connection_revision: Option<i64>,
    eligible: bool,
}

fn conflict() -> AppError {
    AppError::Conflict("Une source externe ou son autorisation a changé pendant le traitement. Actualisez le contexte avant de réessayer.".into())
}

fn stamps(rows: Vec<Row>) -> AppResult<Vec<ObservationAuthority>> {
    let mut result = rows
        .into_iter()
        .map(|row| {
            if !row.eligible {
                return Err(conflict());
            }
            Ok(ObservationAuthority {
                source_kind: row.source_kind,
                source_public_id: row.source_public_id,
                connection_public_id: row.connection_public_id.ok_or_else(conflict)?,
                connection_revision: row.connection_revision.ok_or_else(conflict)?,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    result.sort();
    Ok(result)
}

pub(crate) async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    source_ids: &[Uuid],
) -> AppResult<Vec<ObservationAuthority>> {
    if source_ids.is_empty() {
        return Ok(Vec::new());
    }
    stamps(
        sqlx::query_as(ROWS)
            .bind(source_ids)
            .fetch_all(&mut **tx)
            .await?,
    )
}

/// Project locks precede credential locks everywhere. Callers with additional
/// scope locks must acquire those first in ascending project order.
pub(crate) async fn verify(
    tx: &mut Transaction<'_, Postgres>,
    expected: &[ObservationAuthority],
) -> AppResult<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let source_ids: Vec<_> = expected.iter().map(|s| s.source_public_id).collect();
    let rows: Vec<Row> = sqlx::query_as(ROWS)
        .bind(&source_ids)
        .fetch_all(&mut **tx)
        .await?;
    let project_ids: Vec<_> = rows.iter().map(|r| r.project_id).collect();
    sqlx::query("select id from app.projects where id=any($1) order by id for update")
        .bind(project_ids)
        .fetch_all(&mut **tx)
        .await?;
    let mut connections: Vec<_> = expected.iter().map(|s| s.connection_public_id).collect();
    connections.sort_unstable();
    connections.dedup();
    for connection in connections {
        sqlx::query("select pg_advisory_xact_lock_shared(hashtextextended(app.current_workspace_id()::text||':work-tool:'||$1::uuid::text,0))")
            .bind(connection).execute(&mut **tx).await?;
    }
    let actual = capture(tx, &source_ids).await?;
    let mut expected = expected.to_vec();
    expected.sort();
    if actual != expected {
        return Err(conflict());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_or_revoked_receipt_cannot_attest_model_input() {
        let row = |eligible, connection_public_id, connection_revision| Row {
            source_kind: "publication_observation".into(),
            source_public_id: Uuid::new_v4(),
            project_id: 1,
            connection_public_id,
            connection_revision,
            eligible,
        };
        assert!(stamps(vec![row(false, Some(Uuid::new_v4()), Some(1))]).is_err());
        assert!(stamps(vec![row(true, None, None)]).is_err());
        assert_eq!(
            stamps(vec![row(true, Some(Uuid::new_v4()), Some(2))]).unwrap()[0].connection_revision,
            2
        );
    }
}
