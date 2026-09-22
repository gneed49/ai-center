use std::collections::HashSet;

use super::models::{ARTIFACT_TYPES, CreateArtifact, ListArtifacts, SetDestination};
use crate::error::{AppError, AppResult};

pub(super) fn artifact_type(value: &str) -> AppResult<()> {
    if ARTIFACT_TYPES.contains(&value) {
        Ok(())
    } else {
        Err(AppError::Invalid("Unsupported artifact type".into()))
    }
}

pub(super) fn content(input: &CreateArtifact) -> AppResult<()> {
    artifact_type(&input.artifact_type)?;
    if !(1..=200).contains(&input.title.trim().chars().count())
        || input.body_markdown.len() > 262_144
        || !input.structured_content.is_object()
        || input.structured_content.to_string().len() > 131_072
        || input.sources.len() > 100
    {
        return Err(AppError::Invalid(
            "Artifact title, content or sources exceed the supported limits".into(),
        ));
    }
    let mut ids = HashSet::new();
    for source in &input.sources {
        if !matches!(
            source.kind.as_str(),
            "knowledge" | "context_pack" | "deliverable" | "session"
        ) || source.public_id.is_nil()
            || !ids.insert((&source.kind, source.public_id))
        {
            return Err(AppError::Invalid(
                "Sources must be supported, unique immutable identities".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn pagination(input: &ListArtifacts) -> AppResult<(i64, i64)> {
    let limit = input.limit.unwrap_or(25);
    let offset = input.offset.unwrap_or(0);
    if !(1..=100).contains(&limit) || !(0..=1_000_000).contains(&offset) {
        return Err(AppError::Invalid(
            "Use limit 1–100 and offset 0–1000000".into(),
        ));
    }
    Ok((limit, offset))
}

pub(super) fn filters(input: &ListArtifacts) -> AppResult<()> {
    if let Some(kind) = &input.artifact_type {
        artifact_type(kind)?;
    }
    if input.q.as_ref().is_some_and(|q| q.chars().count() > 200)
        || input
            .status
            .as_deref()
            .is_some_and(|s| !matches!(s, "draft" | "validated"))
    {
        return Err(AppError::Invalid(
            "Invalid artifact search or status".into(),
        ));
    }
    Ok(())
}

pub(super) fn destination(input: &SetDestination) -> AppResult<()> {
    artifact_type(&input.artifact_type)?;
    if input.expected_revision < 0 || input.label.chars().count() > 120 {
        return Err(AppError::Invalid(
            "Invalid destination revision or label".into(),
        ));
    }
    let target = input.target_id.as_deref().unwrap_or("");
    let valid = match input.provider.as_str() {
        "internal" => input.target_id.is_none(),
        "notion" | "linear" => uuid::Uuid::parse_str(target).is_ok_and(|id| !id.is_nil()),
        "github" => {
            let parts = target.split('/').collect::<Vec<_>>();
            parts.len() == 2
                && parts.iter().all(|part| {
                    !part.is_empty()
                        && part.len() <= 100
                        && *part != "."
                        && *part != ".."
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
                })
        }
        _ => false,
    };
    if !valid {
        return Err(AppError::Invalid(
            "Choose internal storage, a Notion/Linear UUID, or a GitHub owner/repository".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::models::SourceInput;
    use serde_json::json;
    use uuid::Uuid;

    fn draft() -> CreateArtifact {
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Conservation".into(),
            body_markdown: String::new(),
            structured_content: json!({}),
            sources: vec![],
        }
    }

    #[test]
    fn rejects_forged_sources_unknown_kinds_and_invalid_content() {
        let mut value = draft();
        assert!(content(&value).is_ok());
        value.sources = vec![SourceInput {
            kind: "knowledge".into(),
            public_id: Uuid::nil(),
        }];
        assert!(content(&value).is_err());
        let source = SourceInput {
            kind: "knowledge".into(),
            public_id: Uuid::new_v4(),
        };
        value.sources = vec![source.clone(), source];
        assert!(content(&value).is_err());
        value.sources.clear();
        value.structured_content = json!(["not an object"]);
        assert!(content(&value).is_err());
        value.structured_content = json!({});
        value.title = "  ".into();
        assert!(content(&value).is_err());
    }

    #[test]
    fn bounds_search_and_pagination() {
        assert_eq!(pagination(&ListArtifacts::default()).unwrap(), (25, 0));
        assert!(
            pagination(&ListArtifacts {
                limit: Some(101),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            pagination(&ListArtifacts {
                offset: Some(-1),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            filters(&ListArtifacts {
                status: Some("published".into()),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            filters(&ListArtifacts {
                q: Some("x".repeat(201)),
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn destination_accepts_identifiers_not_arbitrary_urls_or_credentials() {
        let mut value = SetDestination {
            artifact_type: "specification".into(),
            provider: "internal".into(),
            target_id: None,
            label: String::new(),
            expected_revision: 0,
        };
        assert!(destination(&value).is_ok());
        value.target_id = Some("unexpected".into());
        assert!(destination(&value).is_err());
        value.provider = "github".into();
        value.target_id = Some("owner/repository".into());
        assert!(destination(&value).is_ok());
        for bad in [
            "https://github.com/owner/repo",
            "owner/repo?token=secret",
            "../repo",
            "owner/repo/path",
        ] {
            value.target_id = Some(bad.into());
            assert!(destination(&value).is_err());
        }
        value.provider = "notion".into();
        value.target_id = Some(Uuid::new_v4().to_string());
        assert!(destination(&value).is_ok());
        value.provider = "linear".into();
        assert!(destination(&value).is_ok());
    }
}
