use super::*;
use ai_center_server::{
    artifacts::{self, CreateArtifact, ValidateArtifact},
    models::SendMessage,
};

#[tokio::test]
#[allow(clippy::too_many_lines)] // One bounded large-corpus lifecycle.
async fn company_chat_retrieves_across_large_scopes_without_claiming_exhaustive_coverage()
-> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Grand projet").await?;
    let (other, _) = ready_project(&foreign, "[FICTIF] Société étrangère").await?;
    let (_, foreign_version) = fixture_knowledge(
        &foreign,
        other,
        "business_rule",
        "[FICTIF] quartzorion reste privé",
    )
    .await?;
    let (_, relevant) = fixture_knowledge(
        &owner,
        project,
        "business_rule",
        "[FICTIF] quartzorion conserve les décisions cent ans.",
    )
    .await?;
    for n in 0..165 {
        fixture_knowledge(
            &owner,
            project,
            "decision",
            &format!("[FICTIF] Décision secondaire numéro {n}"),
        )
        .await?;
    }
    for n in 0..22 {
        let draft = artifacts::create(
            &owner,
            project,
            CreateArtifact {
                artifact_type: "specification".into(),
                title: format!("[FICTIF] Document {n}"),
                body_markdown: format!("Spécification secondaire {n}"),
                structured_content: json!({}),
                sources: vec![],
            },
            None,
        )
        .await?;
        artifacts::validate(
            &owner,
            draft.artifact.public_id,
            ValidateArtifact {
                expected_version_id: draft.current_version.public_id,
            },
            None,
        )
        .await?;
    }
    // Compiler obligations still fail explicitly instead of silently truncating.
    assert!(compile_for(&owner, project, session, "tech").await.is_err());
    let company = company::overview(&owner)
        .await?
        .company_scope
        .context("company scope")?
        .project_public_id;
    let (_, mandatory) = fixture_knowledge(
        &owner,
        company,
        "business_rule",
        "[FICTIF] Toujours obtenir un accord humain.",
    )
    .await?;
    let chat = service::create_session(
        &owner,
        company,
        CreateSession {
            node_key: "general".into(),
            title: None,
        },
    )
    .await?;
    let response = service::send_message(
        &owner,
        chat.session.public_id,
        SendMessage {
            content: "Quelle règle quartzorion existe dans la société ?".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    let metadata = &response
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .context("assistant response")?
        .metadata;
    assert_eq!(metadata["retrieval"]["exhaustive"], false);
    assert!(
        metadata["retrieval"]["omitted_sources"]
            .as_i64()
            .context("omissions")?
            > 0
    );
    let provenance = metadata["source_provenance"]
        .as_array()
        .context("source provenance")?;
    assert!(
        provenance
            .iter()
            .any(|s| s["source_version_public_id"] == relevant.to_string())
    );
    assert!(
        provenance
            .iter()
            .any(|s| s["source_version_public_id"] == mandatory.to_string())
    );
    assert!(
        !provenance
            .iter()
            .any(|s| s["source_version_public_id"] == foreign_version.to_string())
    );
    for n in 0..160 {
        fixture_knowledge(
            &owner,
            company,
            "constraint",
            &format!("[FICTIF] Obligation supplémentaire {n}"),
        )
        .await?;
    }
    let overflow = service::send_message(
        &owner,
        chat.session.public_id,
        SendMessage {
            content: "Continuer".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await;
    assert!(
        matches!(overflow,Err(AppError::Invalid(message)) if message.contains("règles obligatoires"))
    );
    Ok(())
}
