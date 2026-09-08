//! Offline compiler contracts using explicitly fictional project material.
//! These checks do not evaluate the semantic quality of a provider or model.

use ai_center_server::{
    context::{ContextCandidate, compile_context_pack},
    error::AppError,
};
use serde_json::{Value, json};
use uuid::Uuid;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/synthetic-projects.json"))
        .expect("versioned synthetic fixtures must parse")
}

fn candidate(fixture: &Value, field: &str, entry_type: &str, node_key: &str) -> ContextCandidate {
    ContextCandidate {
        knowledge_public_id: Uuid::new_v4(),
        version_public_id: Uuid::new_v4(),
        version_number: 1,
        entry_type: entry_type.into(),
        title: fixture["name"].as_str().unwrap().into(),
        statement: fixture[field].as_str().unwrap().into(),
        rationale: "Cas fictif explicitement autorisé pour les tests.".into(),
        node_key: node_key.into(),
    }
}

#[test]
fn fictional_policies_keep_their_versions_and_optional_history_stays_optional() {
    for fixture in fixtures() {
        let requirement = candidate(&fixture, "rule", "requirement", "product");
        let historical = candidate(
            &fixture,
            "historical_technical_note",
            "technical_rule",
            "tech",
        );
        let candidates = [requirement.clone(), historical.clone()];
        let compile = |selected: &[Uuid]| {
            compile_context_pack(
                fixture["objective"].as_str().unwrap(),
                "Projet fictif",
                7,
                &json!({"contract_key": "technical-delivery-plan"}),
                &candidates,
                selected,
                12_000,
                "hybrid",
            )
            .expect("the policy must fit with or without optional context")
        };
        let minimum = compile(&[]);
        assert_eq!(minimum.included_count, 1);
        assert_eq!(
            minimum.content["knowledge"][0]["statement"],
            fixture["rule"]
        );
        assert_eq!(minimum.content["knowledge"][0]["version_number"], 1);
        assert_eq!(minimum.source_graph_version, 7);
        assert_eq!(minimum.content["graph_version"], 7);
        assert_eq!(minimum.selection_items[0].reason_code, "contract_required");
        assert_eq!(
            minimum.selection_items[1].reason_code,
            "historical_tech_unrelated"
        );
        assert!(!minimum.selection_items[1].included);
        assert!(!minimum.selection_items[1].required);

        let with_history = compile(&[historical.version_public_id]);
        assert_eq!(with_history.included_count, 2);
        assert!(with_history.selection_items[1].included);
        assert!(!with_history.selection_items[1].required);
        assert_eq!(
            with_history.selection_items[1].reason_code,
            "semantic_relevance"
        );
        assert_eq!(
            with_history.content["provenance"][1]["version_public_id"],
            json!(historical.version_public_id)
        );
        assert_eq!(
            with_history.content["provenance"][0]["knowledge_public_id"],
            json!(requirement.knowledge_public_id)
        );
        assert_ne!(minimum.content_hash, with_history.content_hash);
    }
}

#[test]
fn a_selected_long_history_cannot_displace_either_projects_policy() {
    for fixture in fixtures() {
        let requirement = candidate(&fixture, "rule", "requirement", "product");
        let mut historical = candidate(
            &fixture,
            "historical_technical_note",
            "technical_rule",
            "tech",
        );
        historical.statement = historical.statement.repeat(100);
        let pack = compile_context_pack(
            fixture["objective"].as_str().unwrap(),
            "Projet fictif",
            7,
            &json!({}),
            &[historical.clone(), requirement.clone()],
            &[historical.version_public_id],
            500,
            "hybrid",
        )
        .expect("the short mandatory policy fits despite a large optional item listed first");
        assert_eq!(pack.included_count, 1);
        assert!(pack.token_count <= 500);
        assert_eq!(pack.selection_items[0].reason_code, "budget_exceeded");
        assert!(!pack.selection_items[0].included);
        assert!(pack.selection_items[1].included);
        assert!(pack.selection_items[1].required);
        assert_eq!(
            pack.content["knowledge"][0]["version_public_id"],
            json!(requirement.version_public_id)
        );
    }
}

#[test]
fn a_selector_cannot_add_a_source_from_the_other_fictional_project() {
    let cases = fixtures();
    let library = candidate(&cases[0], "rule", "requirement", "product");
    let workshop = candidate(&cases[1], "rule", "requirement", "product");
    for (own, foreign) in [(&library, &workshop), (&workshop, &library)] {
        let result = compile_context_pack(
            "Projet fictif",
            "",
            1,
            &json!({}),
            std::slice::from_ref(own),
            &[foreign.version_public_id],
            12_000,
            "hybrid",
        );
        assert!(
            matches!(result, Err(AppError::Invalid(message)) if message.contains("unknown source"))
        );
    }
}
