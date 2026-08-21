use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{AgentTurn, ProposalDraft},
};

#[derive(Clone)]
pub struct AgentInput {
    pub scope_kind: String,
    pub instructions: String,
    pub user_message: String,
    pub context: Value,
}

#[async_trait]
pub trait AgentEngine: Send + Sync {
    async fn respond(&self, input: AgentInput) -> AppResult<AgentTurn>;
}

pub struct OpenAiEngine {
    client: Client,
    api_key: SecretString,
    model: String,
}

impl OpenAiEngine {
    #[must_use]
    pub fn new(api_key: SecretString, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl AgentEngine for OpenAiEngine {
    async fn respond(&self, input: AgentInput) -> AppResult<AgentTurn> {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["response", "proposals", "sources"],
            "properties": {
                "response": {"type": "string"},
                "sources": {"type": "array", "items": {"type": "string", "format": "uuid"}},
                "proposals": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["entry_type", "title", "statement", "rationale"],
                        "properties": {
                            "entry_type": {"type": "string", "enum": ["decision", "business_rule", "technical_rule", "requirement", "acceptance_criterion", "constraint", "open_question"]},
                            "title": {"type": "string"},
                            "statement": {"type": "string"},
                            "rationale": {"type": "string"}
                        }
                    }
                }
            }
        });
        let body = json!({
            "model": self.model,
            "instructions": format!(
                "{}\nTu opères dans le scope {}. Réponds en français. Propose des mutations atomiques mais ne les confirme jamais. Cite uniquement des UUID présents dans le contexte.",
                input.instructions, input.scope_kind
            ),
            "input": [{
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": format!("CONTEXTE:\n{}\n\nMESSAGE:\n{}", input.context, input.user_message)
                }]
            }],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "ai_center_agent_turn",
                    "strict": true,
                    "schema": schema
                }
            }
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(self.api_key.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|error| AppError::Agent(error.to_string()))?;
        let status = response.status();
        let value: Value = response
            .json()
            .await
            .map_err(|error| AppError::Agent(error.to_string()))?;
        if !status.is_success() {
            let message = value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("OpenAI request failed");
            return Err(AppError::Agent(message.into()));
        }

        parse_agent_turn(&value)
    }
}

fn parse_agent_turn(value: &Value) -> AppResult<AgentTurn> {
    let output = value
        .get("output")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        })
        .and_then(|item| item.get("content"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("output_text"))
        })
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Agent("OpenAI returned no structured output".into()))?;

    serde_json::from_str(output).map_err(|error| AppError::Agent(error.to_string()))
}

#[derive(Default)]
pub struct DeterministicEngine;

#[async_trait]
impl AgentEngine for DeterministicEngine {
    async fn respond(&self, input: AgentInput) -> AppResult<AgentTurn> {
        Ok(deterministic_turn(
            &input.scope_kind,
            &input.user_message,
            &input.context,
        ))
    }
}

fn deterministic_turn(scope: &str, message: &str, context: &Value) -> AgentTurn {
    let normalized = message.to_lowercase();
    let sources = context
        .get("knowledge")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("version_public_id").and_then(Value::as_str))
        .filter_map(|id| id.parse::<Uuid>().ok())
        .collect();

    if scope == "product" {
        let never_expires = normalized.contains("n'expir")
            || normalized.contains("n’expir")
            || normalized.contains("jamais");
        let expiration = if never_expires {
            "n'expirent jamais"
        } else {
            "suivent la règle d'expiration confirmée"
        };
        AgentTurn {
            response: "J’ai transformé cette intention en unités vérifiables. Vérifiez chaque proposition avant de l’inscrire au graphe.".into(),
            proposals: vec![
                proposal("business_rule", "Durée de validité des crédits", format!("Les crédits achetés {expiration}."), "La durée de validité est une règle métier structurante."),
                proposal("requirement", "Solde de crédits fiable", "Le système doit afficher et débiter le solde de crédits achetés sans modifier leur durée de validité.", "Cette exigence rend la règle observable dans le produit."),
                proposal("acceptance_criterion", "Conservation du solde", "Étant donné un achat de crédits confirmé, lorsque 90 jours se sont écoulés, alors le solde restant est toujours disponible.", "Le critère couvre explicitement le seuil de 90 jours."),
            ],
            sources,
        }
    } else {
        let purge = normalized.contains("90") || normalized.contains("purge");
        let statement = if purge {
            "Le stockage doit purger automatiquement les crédits non consommés après 90 jours."
        } else {
            "Le ledger de crédits doit conserver un historique immuable et idempotent."
        };
        AgentTurn {
            response: "Le contexte transmis suffit : je propose une décision technique atomique, sans reformuler le besoin Produit.".into(),
            proposals: vec![proposal(
                "technical_rule",
                if purge { "Purge après 90 jours" } else { "Ledger immuable" },
                statement,
                "La règle sera reliée au plan technique et à ses preuves.",
            )],
            sources,
        }
    }
}

fn proposal(
    entry_type: &str,
    title: &str,
    statement: impl Into<String>,
    rationale: &str,
) -> ProposalDraft {
    ProposalDraft {
        entry_type: entry_type.into(),
        title: title.into(),
        statement: statement.into(),
        rationale: rationale.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_turn_proposes_rule_requirement_and_criterion() {
        let turn = deterministic_turn("product", "Les crédits n'expirent jamais", &json!({}));
        assert_eq!(turn.proposals.len(), 3);
        assert!(
            turn.proposals
                .iter()
                .any(|item| item.entry_type == "business_rule")
        );
        assert!(
            turn.proposals
                .iter()
                .any(|item| item.entry_type == "requirement")
        );
        assert!(
            turn.proposals
                .iter()
                .any(|item| item.entry_type == "acceptance_criterion")
        );
    }

    #[test]
    fn tech_turn_can_express_the_reference_conflict() {
        let turn = deterministic_turn("tech", "On purge après 90 jours", &json!({}));
        assert!(turn.proposals[0].statement.contains("90 jours"));
    }

    #[test]
    fn parses_the_responses_api_structured_output_contract() {
        let response = json!({
            "output": [{
                "type": "message",
                "content": [{
                    "type": "output_text",
                    "text": "{\"response\":\"Contexte cadré\",\"proposals\":[],\"sources\":[]}"
                }]
            }]
        });
        let turn = parse_agent_turn(&response).expect("structured response should parse");
        assert_eq!(turn.response, "Contexte cadré");
        assert!(turn.proposals.is_empty());
    }
}
