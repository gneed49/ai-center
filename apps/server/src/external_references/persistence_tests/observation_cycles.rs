use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise_cycles(
    state: &AppState,
    context: &RequestContext,
    github: &GitHubRuntime,
    mode: &AtomicU16,
    project_id: Uuid,
    connection: Uuid,
    evidence_input: CreateExternalEvidence,
) -> anyhow::Result<()> {
    mode.store(200, Ordering::Relaxed);
    let first = create_github_reference(
        state,
        context,
        github,
        project_id,
        Uuid::new_v4(),
        CreateGitHubReference {
            url: "https://github.com/acme/context/pull/19".into(),
            tool_connection_id: Some(connection),
            tracking: None,
        },
    )
    .await?;
    let reference_id = first.reference.public_id;
    mode.store(201, Ordering::Relaxed);
    let second = refresh(state, context, github, reference_id, Uuid::new_v4()).await?;
    assert_ne!(
        first.latest_observation.as_ref().unwrap().observed_state["head_sha"],
        second.latest_observation.as_ref().unwrap().observed_state["head_sha"]
    );
    mode.store(200, Ordering::Relaxed);
    let returned = refresh(state, context, github, reference_id, Uuid::new_v4()).await?;
    assert_eq!(
        returned.latest_observation.as_ref().unwrap().observed_state["head_sha"],
        HEAD_SHA
    );
    assert_ne!(
        returned.latest_observation.as_ref().unwrap().public_id,
        first.latest_observation.as_ref().unwrap().public_id
    );
    let proof =
        create_evidence(state, context, reference_id, Uuid::new_v4(), evidence_input).await?;
    assert!(
        proof.source_reference.contains(HEAD_SHA),
        "a new proof must bind the restored SHA"
    );

    // A commit's SHA, provider date and ETag remain fixed. Only its mutable
    // status changes success -> pending -> success, reproducing identical A hashes.
    let commit_id = create_github_reference(
        state,
        context,
        github,
        project_id,
        Uuid::new_v4(),
        CreateGitHubReference {
            url: format!("https://github.com/acme/context/commit/{HEAD_SHA}"),
            tool_connection_id: Some(connection),
            tracking: None,
        },
    )
    .await?
    .reference
    .public_id;
    let initial = observations(state, context, commit_id).await?;
    mode.store(202, Ordering::Relaxed);
    let pending = refresh(state, context, github, commit_id, Uuid::new_v4()).await?;
    mode.store(200, Ordering::Relaxed);
    let key = Uuid::new_v4();
    let restored = refresh(state, context, github, commit_id, key).await?;
    let history = observations(state, context, commit_id).await?;
    assert_eq!(
        &history[..initial.len()],
        initial.as_slice(),
        "old observations are immutable"
    );
    assert_eq!(history.len(), initial.len() + 2);
    let latest = restored.latest_observation.as_ref().unwrap();
    assert_eq!(latest.content_hash, initial.last().unwrap().content_hash);
    assert_ne!(
        latest.content_hash,
        pending.latest_observation.as_ref().unwrap().content_hash
    );
    assert_ne!(latest.public_id, initial.last().unwrap().public_id);
    assert_eq!(latest.etag, initial.last().unwrap().etag);
    assert_eq!(history.last(), Some(latest));
    assert_eq!(
        restored,
        refresh(state, context, github, commit_id, key).await?
    );
    refresh(state, context, github, commit_id, Uuid::new_v4()).await?;
    assert_eq!(
        observations(state, context, commit_id).await?,
        history,
        "replay and consecutive unchanged states do not append duplicates"
    );

    for _ in 0..2 {
        mode.store(404, Ordering::Relaxed);
        let missing = refresh(state, context, github, commit_id, Uuid::new_v4()).await?;
        assert_eq!(
            missing.latest_observation.unwrap().observation_status,
            "unavailable"
        );
        mode.store(200, Ordering::Relaxed);
        let recovered = refresh(state, context, github, commit_id, Uuid::new_v4()).await?;
        assert_eq!(
            recovered.latest_observation.unwrap().observation_status,
            "current"
        );
    }
    let recovered_history = observations(state, context, commit_id).await?;
    assert_eq!(&recovered_history[..history.len()], history.as_slice());
    assert_eq!(recovered_history.len(), history.len() + 4);
    Ok(())
}

async fn observations(
    state: &AppState,
    context: &RequestContext,
    reference_id: Uuid,
) -> AppResult<Vec<ExternalReferenceObservationView>> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    let rows = sqlx::query_as::<_, ObservationRecord>(
        "select observation.public_id, observation_status, content_hash, observation.etag, observed_state, provider_updated_at, observed_at
         from app.external_reference_observations observation
         join app.external_references reference on reference.id = observation.external_reference_id
         where reference.public_id = $1 order by observation.id",
    ).bind(reference_id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(ObservationRecord::view).collect())
}
