//! Pure, bounded projection of validated structured tickets into external issues.
use crate::{
    artifacts::generation_contract::{self, ArtifactDraft},
    error::{AppError, AppResult},
};
use serde_json::Value;
use std::fmt::Write as _;
use uuid::Uuid;

pub(super) fn target(provider: &str, value: &str) -> AppResult<String> {
    let valid = match provider {
        "linear" => Uuid::parse_str(value)
            .ok()
            .filter(|id| !id.is_nil())
            .map(|id| id.to_string()),
        "github" => {
            let parts: Vec<_> = value.split('/').collect();
            (parts.len() == 2
                && parts.iter().all(|part| {
                    !part.is_empty()
                        && part.len() <= 100
                        && *part != "."
                        && *part != ".."
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
                }))
            .then(|| value.to_ascii_lowercase())
        }
        _ => None,
    };
    valid.ok_or_else(|| {
        AppError::Invalid(
            "Choisissez une équipe Linear ou un dépôt GitHub valide pour ces tickets.".into(),
        )
    })
}
pub(super) fn indexes(values: &[i16]) -> AppResult<Vec<i16>> {
    let mut result = values.to_vec();
    result.sort_unstable();
    if result.is_empty()
        || result.len() > 30
        || result.iter().any(|i| !(0..30).contains(i))
        || result.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(AppError::Invalid(
            "Sélectionnez de 1 à 30 tickets distincts de cette version.".into(),
        ));
    }
    Ok(result)
}
pub(super) fn draft(kind: &str, content: &Value, sources: &[Value]) -> AppResult<ArtifactDraft> {
    let invalid = || {
        AppError::Invalid(
            "Cette version doit contenir un tableau de tickets au format agent-artifact-v1 valide."
                .into(),
        )
    };
    if !matches!(kind, "product_tickets" | "technical_tickets")
        || content["format"] != "agent-artifact-v1"
        || content["artifact_type"] != kind
    {
        return Err(invalid());
    }
    let draft: ArtifactDraft =
        serde_json::from_value(content["draft"].clone()).map_err(|_| invalid())?;
    let ids: Vec<_> = sources
        .iter()
        .filter(|source| {
            matches!(
                source["kind"].as_str(),
                Some("knowledge" | "artifact_version")
            )
        })
        .filter_map(|source| {
            source["public_id"]
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
        })
        .collect();
    generation_contract::validate(kind, &draft, &ids).map_err(|_| invalid())?;
    Ok(draft)
}
pub(super) fn business_body(
    artifact: Uuid,
    version: Uuid,
    number: i32,
    index: i16,
    draft: &ArtifactDraft,
    sources: &[Value],
) -> AppResult<(String, String)> {
    let ticket = usize::try_from(index)
        .ok()
        .and_then(|i| draft.tickets.get(i))
        .ok_or_else(|| {
            AppError::Invalid("Ce ticket n’existe pas dans la version sélectionnée.".into())
        })?;
    let mut body = format!("{}\n\n## Critères d’acceptation\n", ticket.description);
    for criterion in &ticket.acceptance_criteria {
        let _ = writeln!(body, "- {criterion}");
    }
    let _ = write!(body, "\n## Sources citées\n");
    if ticket.source_ids.is_empty() {
        body.push_str("Aucune source citée pour ce ticket.\n");
    }
    for id in &ticket.source_ids {
        let source = sources
            .iter()
            .find(|s| s["public_id"].as_str() == Some(&id.to_string()))
            .ok_or_else(|| {
                AppError::Invalid("Une source citée n’appartient pas à cette version.".into())
            })?;
        let _ = writeln!(
            body,
            "- {} · {} · version {} · {}",
            source["title"].as_str().unwrap_or("Source"),
            source["kind"].as_str().unwrap_or(""),
            source["version"],
            id
        );
    }
    let _ = write!(
        body,
        "\n## Provenance AI Center\nArtefact : {artifact}\nVersion immuable : {version}\nVersion : {number}\nTicket : {} (index {index})\n",
        index + 1
    );
    // Marker UUIDs always have this fixed length; budget includes separators.
    if final_body(&body, Uuid::nil()).len() > 61_440 {
        return Err(AppError::Invalid(
            "Ce ticket dépasse la limite de publication de 60 Kio.".into(),
        ));
    }
    Ok((ticket.title.clone(), body))
}
pub(super) fn final_body(business: &str, id: Uuid) -> String {
    format!("{business}\n---\n{}\n", super::publications::marker(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::generation_contract::ArtifactGenerationInput;
    use serde_json::json;
    fn fixture() -> ArtifactDraft {
        generation_contract::deterministic(ArtifactGenerationInput {
            artifact_type: "product_tickets".into(),
            instructions: "[FICTIF] Description complète.".into(),
            agent_instructions: String::new(),
            objective: "[FICTIF] Publication".into(),
            context: json!({}),
            conversation: json!([]),
            source_ids: vec![],
        })
        .unwrap()
        .output
    }
    #[test]
    fn selection_is_canonical_but_never_silently_deduplicates_or_accepts_other_tools() {
        assert_eq!(indexes(&[2, 0, 1]).unwrap(), vec![0, 1, 2]);
        for ids in [vec![], vec![1, 1], vec![-1], vec![30], (0..31).collect()] {
            assert!(indexes(&ids).is_err());
        }
        assert_eq!(target("github", "Fictif/Repo").unwrap(), "fictif/repo");
        assert!(target("github", "../repo").is_err());
        assert!(target("notion", &Uuid::new_v4().to_string()).is_err());
    }
    #[test]
    fn publication_uses_only_the_selected_ticket_and_its_citations() {
        let mut data = fixture();
        data.tickets.push(data.tickets[0].clone());
        data.tickets[1].description = "[FICTIF] PRIVATE OTHER TICKET".into();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        data.tickets[0].source_ids = vec![a];
        let sources = vec![
            json!({"public_id":a,"kind":"knowledge","title":"[FICTIF] Rule A","version":1}),
            json!({"public_id":b,"kind":"knowledge","title":"[FICTIF] UNRELATED RULE","version":1}),
        ];
        let content =
            json!({"format":"agent-artifact-v1","artifact_type":"product_tickets","draft":data});
        let checked = draft("product_tickets", &content, &sources).unwrap();
        let (_, body) =
            business_body(Uuid::new_v4(), Uuid::new_v4(), 3, 0, &checked, &sources).unwrap();
        assert!(body.contains("Rule A"));
        assert!(!body.contains("PRIVATE OTHER"));
        assert!(!body.contains("UNRELATED"));
        assert!(!body.contains("AI Center publication:"));
        let id = Uuid::new_v4();
        let final_text = final_body(&body, id);
        assert!(final_text.starts_with(&body));
        assert!(final_text.ends_with(&format!("AI Center publication: {id}\n")));
        assert!(business_body(Uuid::nil(), Uuid::nil(), 3, 2, &checked, &sources).is_err());
        assert!(draft("product_tickets", &content, &[]).is_err());
        assert!(draft("product_tickets", &json!({}), &sources).is_err());
    }
    #[test]
    fn unicode_and_citation_expansion_are_budgeted_with_the_final_marker() {
        let mut data = fixture();
        data.tickets[0].description = "é".repeat(4000);
        data.tickets[0].acceptance_criteria = vec!["à".repeat(1000); 20];
        let sources: Vec<_> = (0..100)
            .map(|_| {
                let id = Uuid::new_v4();
                data.tickets[0].source_ids.push(id);
                json!({"public_id":id,"kind":"knowledge","title":"é".repeat(100),"version":1})
            })
            .collect();
        assert!(business_body(Uuid::new_v4(), Uuid::new_v4(), 1, 0, &data, &sources).is_err());
    }
}
