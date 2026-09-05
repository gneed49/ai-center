use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const DEFAULT_CONTEXT_BUDGET_TOKENS: i32 = 12_000;
pub const COMPILER_VERSION: &str = "alpha-context-compiler-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCandidate {
    pub knowledge_public_id: Uuid,
    pub version_public_id: Uuid,
    pub version_number: i32,
    pub entry_type: String,
    pub title: String,
    pub statement: String,
    pub rationale: String,
    pub node_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSelectionItem {
    pub knowledge_public_id: Uuid,
    pub version_public_id: Uuid,
    pub included: bool,
    pub required: bool,
    pub reason_code: String,
    pub explanation: String,
    pub rank: i32,
    pub token_estimate: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContextPack {
    pub compiler_version: String,
    pub selection_mode: String,
    pub source_graph_version: i64,
    pub token_budget: i32,
    pub token_count: i32,
    pub candidate_count: i32,
    pub included_count: i32,
    pub content_hash: String,
    pub content: Value,
    pub selection_items: Vec<ContextSelectionItem>,
}

/// Compiles a bounded, inspectable Product-to-Tech context projection.
///
/// Contract-required knowledge is selected deterministically. The model may
/// select optional candidates, but it can never inject identifiers that were
/// not offered by the server.
///
/// # Errors
///
/// Returns an error for an invalid budget, an unknown model-selected source,
/// or a context payload that cannot be serialized.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn compile_context_pack(
    objective: &str,
    project_summary: &str,
    source_graph_version: i64,
    contract: &Value,
    candidates: &[ContextCandidate],
    optional_selected_ids: &[Uuid],
    token_budget: i32,
    selection_mode: &str,
) -> AppResult<CompiledContextPack> {
    if token_budget <= 0 {
        return Err(AppError::Invalid(
            "context token budget must be positive".into(),
        ));
    }
    if !matches!(selection_mode, "deterministic" | "hybrid") {
        return Err(AppError::Invalid(
            "context selection mode must be deterministic or hybrid".into(),
        ));
    }

    let known_ids = candidates
        .iter()
        .map(|candidate| candidate.version_public_id)
        .collect::<HashSet<_>>();
    if let Some(unknown) = optional_selected_ids
        .iter()
        .find(|public_id| !known_ids.contains(public_id))
    {
        return Err(AppError::Invalid(format!(
            "context selector returned an unknown source: {unknown}"
        )));
    }
    let optional = optional_selected_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();

    let base_token_count = estimate_tokens(objective)
        .saturating_add(estimate_tokens(project_summary))
        .saturating_add(estimate_json_tokens(contract));
    let mandatory_token_count = candidates
        .iter()
        .filter(|candidate| is_contract_required(candidate))
        .fold(base_token_count, |total, candidate| {
            total.saturating_add(estimate_candidate_tokens(candidate))
        });
    if mandatory_token_count > token_budget {
        return Err(AppError::Invalid(format!(
            "mandatory context requires an estimated {mandatory_token_count} tokens, exceeding the configured budget of {token_budget}"
        )));
    }

    // Reserve every mandatory item before considering optional candidates so
    // an early optional choice can never push a later obligation over budget.
    let mut token_count = mandatory_token_count;
    let mut selected = Vec::new();
    let mut selection_items = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().enumerate() {
        let required = is_contract_required(candidate);
        let selected_by_model = optional.contains(&candidate.version_public_id);
        let estimate = estimate_candidate_tokens(candidate);
        let fits = required || token_count.saturating_add(estimate) <= token_budget;
        let included = required || selected_by_model && fits;

        let (reason_code, explanation) = if required {
            (
                "contract_required",
                "Élément obligatoire pour le contrat de livraison Tech.",
            )
        } else if selected_by_model && fits {
            (
                "semantic_relevance",
                "Élément optionnel retenu pour sa pertinence avec la tâche.",
            )
        } else if selected_by_model {
            (
                "budget_exceeded",
                "Élément pertinent exclu car le budget de contexte est atteint.",
            )
        } else if candidate.node_key == "tech" {
            (
                "historical_tech_unrelated",
                "Historique Tech non requis et non sélectionné pour ce handoff.",
            )
        } else {
            (
                "not_selected",
                "Élément optionnel non retenu pour cette tâche.",
            )
        };

        if included && !required {
            token_count = token_count.saturating_add(estimate);
        }
        if included {
            selected.push(candidate.clone());
        }
        selection_items.push(ContextSelectionItem {
            knowledge_public_id: candidate.knowledge_public_id,
            version_public_id: candidate.version_public_id,
            included,
            required,
            reason_code: reason_code.into(),
            explanation: explanation.into(),
            rank: i32::try_from(index + 1).unwrap_or(i32::MAX),
            token_estimate: estimate,
        });
    }

    let content = json!({
        "objective": objective,
        "project_summary": project_summary,
        "graph_version": source_graph_version,
        "contract": contract,
        "knowledge": selected,
        "provenance": selection_items.iter().filter(|item| item.included).map(|item| json!({
            "knowledge_public_id": item.knowledge_public_id,
            "version_public_id": item.version_public_id,
            "reason_code": item.reason_code,
            "explanation": item.explanation,
        })).collect::<Vec<_>>(),
    });
    let canonical =
        serde_json::to_vec(&content).map_err(|error| AppError::Internal(error.to_string()))?;
    let content_hash = format!("{:x}", Sha256::digest(canonical));
    let included_count = i32::try_from(selection_items.iter().filter(|item| item.included).count())
        .unwrap_or(i32::MAX);

    Ok(CompiledContextPack {
        compiler_version: COMPILER_VERSION.into(),
        selection_mode: selection_mode.into(),
        source_graph_version,
        token_budget,
        token_count,
        candidate_count: i32::try_from(candidates.len()).unwrap_or(i32::MAX),
        included_count,
        content_hash,
        content,
        selection_items,
    })
}

fn is_contract_required(candidate: &ContextCandidate) -> bool {
    matches!(
        candidate.entry_type.as_str(),
        "business_rule" | "requirement" | "acceptance_criterion" | "constraint" | "open_question"
    ) || (candidate.entry_type == "decision" && candidate.node_key == "product")
}

fn estimate_candidate_tokens(candidate: &ContextCandidate) -> i32 {
    estimate_tokens(&candidate.title)
        + estimate_tokens(&candidate.statement)
        + estimate_tokens(&candidate.rationale)
        + 12
}

fn estimate_tokens(value: &str) -> i32 {
    i32::try_from(value.chars().count().div_ceil(4)).unwrap_or(i32::MAX)
}

fn estimate_json_tokens(value: &Value) -> i32 {
    serde_json::to_string(value).map_or(i32::MAX, |serialized| estimate_tokens(&serialized))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(entry_type: &str, node_key: &str, statement: &str) -> ContextCandidate {
        ContextCandidate {
            knowledge_public_id: Uuid::new_v4(),
            version_public_id: Uuid::new_v4(),
            version_number: 1,
            entry_type: entry_type.into(),
            title: entry_type.into(),
            statement: statement.into(),
            rationale: "Test".into(),
            node_key: node_key.into(),
        }
    }

    #[test]
    fn required_product_knowledge_is_included_and_unrelated_tech_is_excluded() {
        let requirement = candidate("requirement", "product", "Le produit doit fonctionner.");
        let historical = candidate("technical_rule", "tech", "Ancienne architecture.");
        let compiled = compile_context_pack(
            "Prouver le handoff",
            "Projet test",
            3,
            &json!({"contract_key": "technical-delivery-plan"}),
            &[requirement.clone(), historical.clone()],
            &[],
            DEFAULT_CONTEXT_BUDGET_TOKENS,
            "deterministic",
        )
        .expect("context should compile");

        assert!(compiled.selection_items[0].included);
        assert!(!compiled.selection_items[1].included);
        assert_eq!(compiled.included_count, 1);
        assert_eq!(compiled.content_hash.len(), 64);
    }

    #[test]
    fn rejects_a_model_selected_uuid_outside_the_candidate_set() {
        let requirement = candidate("requirement", "product", "Le produit doit fonctionner.");
        let error = compile_context_pack(
            "Objectif",
            "Résumé",
            1,
            &json!({}),
            &[requirement],
            &[Uuid::new_v4()],
            DEFAULT_CONTEXT_BUDGET_TOKENS,
            "hybrid",
        )
        .expect_err("unknown sources must be rejected");
        assert!(error.to_string().contains("unknown source"));
    }

    #[test]
    fn rejects_a_budget_that_cannot_hold_mandatory_context() {
        let requirement = candidate("requirement", "product", &"important ".repeat(100));
        let error = compile_context_pack(
            "Objectif",
            "Résumé",
            1,
            &json!({}),
            &[requirement],
            &[],
            10,
            "deterministic",
        )
        .expect_err("mandatory overflow must be diagnosed before persistence");
        assert!(error.to_string().contains("mandatory context requires"));
        assert!(error.to_string().contains("configured budget of 10"));
    }

    #[test]
    fn records_hybrid_mode_even_when_the_model_selects_no_optional_item() {
        let requirement = candidate("requirement", "product", "Le produit doit fonctionner.");
        let compiled = compile_context_pack(
            "Objectif",
            "Résumé",
            1,
            &json!({}),
            &[requirement],
            &[],
            DEFAULT_CONTEXT_BUDGET_TOKENS,
            "hybrid",
        )
        .expect("an empty optional selection is still a model-assisted run");
        assert_eq!(compiled.selection_mode, "hybrid");
    }
}
