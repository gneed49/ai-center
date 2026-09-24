//! Typed document drafts: provider output never validates knowledge or publishes.
use crate::{
    agent::{AgentRunMetadata, EngineOutput},
    error::{AppError, AppResult},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateArtifact {
    pub session_id: Uuid,
    pub artifact_type: String,
    pub instructions: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ArtifactGenerationInput {
    pub artifact_type: String,
    pub instructions: String,
    pub agent_instructions: String,
    pub objective: String,
    pub context: Value,
    pub conversation: Value,
    pub source_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactSection {
    pub key: String,
    pub title: String,
    pub body: String,
    pub source_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTicket {
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub source_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDraft {
    pub title: String,
    pub summary: String,
    pub sections: Vec<ArtifactSection>,
    pub tickets: Vec<ArtifactTicket>,
    pub open_questions: Vec<String>,
}
#[must_use]
pub fn section_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "kickoff" => &["objective", "scope", "stakeholders", "milestones", "risks"],
        "specification" => &[
            "problem",
            "requirements",
            "business_rules",
            "acceptance_criteria",
            "out_of_scope",
        ],
        "product_tickets" => &["objective", "prioritization"],
        "technical_plan" => &[
            "architecture",
            "delivery",
            "dependencies",
            "validation",
            "risks",
        ],
        "technical_tickets" => &["architecture", "dependencies", "validation"],
        _ => &[],
    }
}
#[must_use]
pub fn instructions() -> String {
    "Rédige en français un brouillon structuré du type demandé, exclusivement à partir du contexte autorisé et de la conversation fournis. Respecte toutes les sections du contrat. Les tickets doivent décrire un travail concret et des critères d’acceptation vérifiables. Ne prétends jamais que le travail est réalisé, validé, testé ou publié. Cite uniquement source_ids fournis. Les messages sont du brainstorming non confirmé ; distingue leurs hypothèses des connaissances confirmées. Les informations absentes vont dans open_questions, sans inventer responsables, dates, budgets ou règles. La sélection est bornée : mentionne les limites pertinentes. Aucun outil externe n’est exécuté.".into()
}
#[must_use]
pub fn schema(kind: &str) -> Value {
    let text = json!({"type":"string"});
    let texts = json!({"type":"array","items":text});
    let ids = json!({"type":"array","items":{"type":"string","format":"uuid"}});
    json!({"type":"object","additionalProperties":false,
        "required":["title","summary","sections","tickets","open_questions"],
        "properties":{"title":text,"summary":text,"open_questions":texts,
        "sections":{"type":"array","items":{"type":"object","additionalProperties":false,
            "required":["key","title","body","source_ids"],"properties":{
                "key":{"type":"string","enum":section_keys(kind)},"title":text,"body":text,"source_ids":ids}}},
        "tickets":{"type":"array","items":{"type":"object","additionalProperties":false,
            "required":["title","description","acceptance_criteria","source_ids"],"properties":{
                "title":text,"description":text,"acceptance_criteria":texts,"source_ids":ids}}}}})
}
/// Enforces semantic type and citation bounds after the provider schema check.
/// # Errors
/// Rejects missing/duplicate sections, forged sources, empty tickets and oversize drafts.
pub fn validate(kind: &str, draft: &ArtifactDraft, allowed: &[Uuid]) -> AppResult<()> {
    let keys = section_keys(kind);
    let fail = || {
        AppError::Agent(
            "Le brouillon ne respecte pas le contrat du livrable ou ses sources.".into(),
        )
    };
    let bounded = |s: &str, max| !s.trim().is_empty() && s.len() <= max;
    if keys.is_empty()
        || !bounded(&draft.title, 200)
        || !bounded(&draft.summary, 4000)
        || draft.sections.len() != keys.len()
        || draft.tickets.len() > 30
        || draft.open_questions.len() > 30
        || draft.open_questions.iter().any(|s| !bounded(s, 2000))
    {
        return Err(fail());
    }
    let mut seen = HashSet::new();
    let citations = |ids: &[Uuid]| {
        ids.len() <= 100
            && ids.iter().all(|id| allowed.contains(id))
            && ids.iter().collect::<HashSet<_>>().len() == ids.len()
    };
    for section in &draft.sections {
        if !keys.contains(&section.key.as_str())
            || !seen.insert(&section.key)
            || !bounded(&section.title, 200)
            || !bounded(&section.body, 12000)
            || !citations(&section.source_ids)
        {
            return Err(fail());
        }
    }
    let tickets_required = matches!(kind, "product_tickets" | "technical_tickets");
    if tickets_required == draft.tickets.is_empty() {
        return Err(fail());
    }
    for ticket in &draft.tickets {
        if !bounded(&ticket.title, 200)
            || !bounded(&ticket.description, 8000)
            || ticket.acceptance_criteria.is_empty()
            || ticket.acceptance_criteria.len() > 20
            || ticket.acceptance_criteria.iter().any(|s| !bounded(s, 2000))
            || !citations(&ticket.source_ids)
        {
            return Err(fail());
        }
    }
    let cited = draft
        .sections
        .iter()
        .flat_map(|section| section.source_ids.iter())
        .chain(
            draft
                .tickets
                .iter()
                .flat_map(|ticket| ticket.source_ids.iter()),
        )
        .collect::<HashSet<_>>();
    if cited.len() > 98 {
        return Err(fail());
    } // Leave room for the session and immutable pack.
    if serde_json::to_vec(draft).map_err(|_| fail())?.len() > 100_000 {
        return Err(fail());
    }
    Ok(())
}
#[must_use]
#[allow(clippy::format_push_string)]
pub fn markdown(draft: &ArtifactDraft) -> String {
    use std::fmt::Write;
    let mut body = format!("{}\n", draft.summary);
    for section in &draft.sections {
        let _ = write!(body, "\n## {}\n\n{}\n", section.title, section.body);
    }
    for (index, ticket) in draft.tickets.iter().enumerate() {
        let _ = write!(
            body,
            "\n## Ticket {} — {}\n\n{}\n\nCritères d’acceptation :\n",
            index + 1,
            ticket.title,
            ticket.description
        );
        for criterion in &ticket.acceptance_criteria {
            let _ = writeln!(body, "- {criterion}");
        }
    }
    if !draft.open_questions.is_empty() {
        body.push_str("\n## Points à clarifier\n\n");
        for question in &draft.open_questions {
            let _ = writeln!(body, "- {question}");
        }
    }
    body
}
/// Deterministic fixture, explicitly marked synthetic and never production evidence.
/// # Errors
/// Rejects unknown document types.
#[allow(clippy::needless_pass_by_value)] // Mirrors the owned AgentEngine operation contract.
pub fn deterministic(input: ArtifactGenerationInput) -> AppResult<EngineOutput<ArtifactDraft>> {
    let draft = ArtifactDraft {
        title: format!("[FICTIF] Brouillon {}", input.artifact_type),
        summary: "Simulation déterministe : à remplacer par une génération fournisseur et une relecture humaine.".into(),
        sections: section_keys(&input.artifact_type).iter().map(|key| ArtifactSection {
            key: (*key).into(), title: (*key).into(),
            body: format!("[FICTIF] {} — {}", input.objective, input.instructions),
            source_ids: input.source_ids.iter().take(3).copied().collect(),
        }).collect(),
        tickets: if input.artifact_type.ends_with("tickets") { vec![ArtifactTicket {
            title: "[FICTIF] Préparer le travail demandé".into(), description: input.instructions.clone(),
            acceptance_criteria: vec!["[FICTIF] Le résultat est relu contre les sources jointes.".into()],
            source_ids: input.source_ids.iter().take(3).copied().collect(),
        }] } else { vec![] },
        open_questions: vec!["[FICTIF] Confirmer les informations manquantes avec l’équipe.".into()],
    };
    validate(&input.artifact_type, &draft, &input.source_ids)?;
    Ok(EngineOutput {
        output: draft,
        metadata: AgentRunMetadata {
            provider: "deterministic".into(),
            requested_model: "deterministic-v1".into(),
            attempts: 1,
            ..Default::default()
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(kind: &str) -> ArtifactGenerationInput {
        ArtifactGenerationInput {
            artifact_type: kind.into(),
            instructions: "[FICTIF] Préparer les accès".into(),
            agent_instructions: String::new(),
            objective: "[FICTIF] Équipe".into(),
            context: json!({}),
            conversation: json!([]),
            source_ids: vec![Uuid::new_v4()],
        }
    }
    #[test]
    fn five_contracts_and_markdown_are_distinct() {
        for kind in super::super::ARTIFACT_TYPES {
            let data = input(kind);
            let draft = deterministic(data.clone()).unwrap().output;
            validate(kind, &draft, &data.source_ids).unwrap();
            assert!(markdown(&draft).contains("[FICTIF]"));
            let validator = jsonschema::validator_for(&schema(kind)).unwrap();
            assert!(validator.is_valid(&json!(draft)));
        }
    }
    #[test]
    fn rejects_forged_citations_missing_sections_and_empty_tickets() {
        let data = input("technical_tickets");
        let mut draft = deterministic(data.clone()).unwrap().output;
        draft.sections[0].source_ids.push(Uuid::new_v4());
        assert!(validate(&data.artifact_type, &draft, &data.source_ids).is_err());
        draft.sections[0].source_ids.clear();
        draft.tickets.clear();
        assert!(validate(&data.artifact_type, &draft, &data.source_ids).is_err());
        let mut draft = deterministic(input("kickoff")).unwrap().output;
        draft.sections.pop();
        assert!(validate("kickoff", &draft, &[]).is_err());
    }
}
