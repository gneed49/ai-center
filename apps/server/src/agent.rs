#[cfg(test)]
use std::time::Duration;

use async_trait::async_trait;
use secrecy::SecretString;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    integrations::openai::OpenAiResponsesClient,
    models::{AgentTurn, ProposalDraft},
};

#[derive(Clone)]
pub struct AgentInput {
    pub scope_kind: String,
    pub instructions: String,
    pub user_message: String,
    pub context: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRunMetadata {
    pub provider: String,
    pub requested_model: String,
    pub served_model: Option<String>,
    pub provider_response_id: Option<String>,
    pub provider_request_id: Option<String>,
    pub status: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    /// Provider-estimated cost in USD when a calibrated price table is
    /// available. `None` is intentional when the runtime cannot make a
    /// defensible estimate for the served model.
    pub estimated_cost: Option<f64>,
    pub latency_ms: i64,
    pub attempts: i32,
}

#[derive(Debug, Clone)]
pub struct EngineOutput<T> {
    pub output: T,
    pub metadata: AgentRunMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSelectionInput {
    pub objective: String,
    pub task_kind: String,
    pub candidates: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSelectionDraft {
    #[serde(default)]
    pub selected_version_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TechnicalPlanInput {
    pub objective: String,
    pub context_pack: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalPlanSectionDraft {
    pub section_key: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub source_version_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalPlanCoverageDraft {
    pub requirement_version_public_id: Uuid,
    pub status: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalPlanDraft {
    pub title: String,
    pub summary: String,
    pub architecture: String,
    pub delivery_slices: Vec<TechnicalPlanSectionDraft>,
    pub risks: Vec<String>,
    pub validation: Vec<String>,
    pub coverage: Vec<TechnicalPlanCoverageDraft>,
}

/// Independent review of a generated plan against its exact immutable pack.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageEvaluationInput {
    pub context_pack: Value,
    pub technical_plan: TechnicalPlanDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCoverageAssessment {
    pub requirement_version_public_id: Uuid,
    pub source_version_ids: Vec<Uuid>,
    pub section_keys: Vec<String>,
    /// Describes the plan only; this is never an evidence validity status.
    pub assessment: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEvaluationDraft {
    pub requirements: Vec<RequirementCoverageAssessment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StewardInput {
    pub candidate_pairs: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionAssessmentDraft {
    pub source_version_ids: Vec<Uuid>,
    pub verdict: String,
    pub severity: String,
    pub confidence: f64,
    pub title: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardOutput {
    pub assessments: Vec<ContradictionAssessmentDraft>,
}

#[async_trait]
pub trait AgentEngine: Send + Sync {
    fn provider_name(&self) -> &'static str;
    fn requested_model(&self) -> &str;

    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>>;
    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>>;
    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>>;
    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>>;
    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>>;
}

pub struct OpenAiEngine {
    client: OpenAiResponsesClient,
    model: String,
}

impl OpenAiEngine {
    /// Builds the hardened HTTP client used for provider calls.
    ///
    /// # Errors
    ///
    /// Returns an internal configuration error instead of silently falling
    /// back to a client without the declared timeout policy.
    pub fn new(api_key: SecretString, model: String) -> AppResult<Self> {
        Ok(Self {
            client: OpenAiResponsesClient::new(api_key)?,
            model,
        })
    }

    #[cfg(test)]
    pub(crate) fn new_for_local_development(
        api_key: SecretString,
        model: String,
        endpoint: &str,
        timeout: Duration,
        retry_delay: Duration,
    ) -> AppResult<Self> {
        Ok(Self {
            client: OpenAiResponsesClient::new_for_local_development(
                api_key,
                endpoint,
                timeout,
                retry_delay,
            )?,
            model,
        })
    }

    async fn request_structured<T: DeserializeOwned>(
        &self,
        operation_name: &str,
        instructions: String,
        input: Value,
        schema: Value,
    ) -> AppResult<EngineOutput<T>> {
        let response = self
            .client
            .request_structured(&self.model, operation_name, &instructions, &input, &schema)
            .await?;
        Ok(EngineOutput {
            output: response.output,
            metadata: AgentRunMetadata {
                provider: "openai".into(),
                requested_model: self.model.clone(),
                served_model: response.metadata.served_model,
                provider_response_id: response.metadata.provider_response_id,
                provider_request_id: response.metadata.provider_request_id,
                status: response.metadata.status,
                input_tokens: response.metadata.input_tokens,
                output_tokens: response.metadata.output_tokens,
                estimated_cost: None,
                latency_ms: response.metadata.latency_ms,
                attempts: i32::try_from(response.metadata.attempts).unwrap_or(i32::MAX),
            },
        })
    }
}

#[async_trait]
impl AgentEngine for OpenAiEngine {
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn requested_model(&self) -> &str {
        &self.model
    }

    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
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
        self.request_structured(
            "ai_center_agent_turn",
            format!(
                "{}\nTu opères dans le scope {}. Réponds en français. Propose des mutations atomiques mais ne les confirme jamais. Cite uniquement des UUID présents dans le contexte.",
                input.instructions, input.scope_kind
            ),
            json!({"context": input.context, "message": input.user_message}),
            schema,
        )
        .await
    }

    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        self.request_structured(
            "ai_center_context_selection",
            "Sélectionne seulement les connaissances optionnelles utiles pour accomplir la tâche. Les éléments contractuellement obligatoires sont ajoutés par le serveur. Retourne uniquement des UUID présents dans candidates; n'invente aucune source."
                .into(),
            serde_json::to_value(input).map_err(|error| AppError::Internal(error.to_string()))?,
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["selected_version_ids"],
                "properties": {
                    "selected_version_ids": {
                        "type": "array",
                        "items": {"type": "string", "format": "uuid"},
                        "uniqueItems": true
                    }
                }
            }),
        )
        .await
    }

    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        self.request_structured(
            "ai_center_technical_plan",
            "Produis un plan de livraison technique concret fondé exclusivement sur le ContextPack. Ne prétends pas qu'un test, un artefact ou du code existe. Chaque section cite seulement les UUID de versions présents dans le pack. La couverture est missing tant qu'aucune preuve externe valide n'est fournie. Aucun mobile ni Android."
                .into(),
            serde_json::to_value(input).map_err(|error| AppError::Internal(error.to_string()))?,
            technical_plan_schema(),
        )
        .await
    }

    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        self.request_structured(
            "ai_center_coverage_assessment",
            "Évalue exclusivement le plan fourni contre les exigences du ContextPack. Retourne exactement une évaluation par version d'exigence du pack. Cite cette exigence et uniquement des versions du pack, sans doublons. Associe uniquement les section_key des delivery_slices du plan. Classe planned si le plan traite l'exigence, partial si des lacunes restent, unaddressed sans section correspondante. Explique les lacunes en français. Une appréciation du plan ne constitue jamais une preuve, ne valide aucun résultat externe et ne signifie pas que le travail est exécuté."
                .into(),
            serde_json::to_value(input).map_err(|error| AppError::Internal(error.to_string()))?,
            coverage_evaluation_schema(),
        )
        .await
    }

    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        self.request_structured(
            "ai_center_steward_assessment",
            "Évalue chaque paire sans inventer de source. Classe-la contradiction, compatible ou ambiguous. Une contradiction doit être directement soutenue par les deux versions citées. Réponds en français."
                .into(),
            serde_json::to_value(input).map_err(|error| AppError::Internal(error.to_string()))?,
            steward_schema(),
        )
        .await
    }
}

fn technical_plan_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["title", "summary", "architecture", "delivery_slices", "risks", "validation", "coverage"],
        "properties": {
            "title": {"type": "string"},
            "summary": {"type": "string"},
            "architecture": {"type": "string"},
            "delivery_slices": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["section_key", "title", "body", "source_version_ids"],
                    "properties": {
                        "section_key": {"type": "string"},
                        "title": {"type": "string"},
                        "body": {"type": "string"},
                        "source_version_ids": {"type": "array", "items": {"type": "string", "format": "uuid"}, "uniqueItems": true}
                    }
                }
            },
            "risks": {"type": "array", "items": {"type": "string"}},
            "validation": {"type": "array", "items": {"type": "string"}},
            "coverage": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["requirement_version_public_id", "status", "explanation"],
                    "properties": {
                        "requirement_version_public_id": {"type": "string", "format": "uuid"},
                        "status": {"type": "string", "enum": ["missing", "partial", "covered"]},
                        "explanation": {"type": "string"}
                    }
                }
            }
        }
    })
}

fn coverage_evaluation_schema() -> Value {
    json!({
        "type": "object", "additionalProperties": false,
        "required": ["requirements"],
        "properties": { "requirements": {
            "type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["requirement_version_public_id", "source_version_ids", "section_keys", "assessment", "explanation"],
                "properties": {
                    "requirement_version_public_id": {"type": "string", "format": "uuid"},
                    "source_version_ids": {"type": "array", "items": {"type": "string", "format": "uuid"}, "uniqueItems": true},
                    "section_keys": {"type": "array", "items": {"type": "string"}, "uniqueItems": true},
                    "assessment": {"type": "string", "enum": ["planned", "partial", "unaddressed"]},
                    "explanation": {"type": "string"}
                }
            }
        }}
    })
}

fn steward_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["assessments"],
        "properties": {
            "assessments": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["source_version_ids", "verdict", "severity", "confidence", "title", "explanation"],
                    "properties": {
                        "source_version_ids": {"type": "array", "minItems": 2, "maxItems": 2, "items": {"type": "string", "format": "uuid"}},
                        "verdict": {"type": "string", "enum": ["contradiction", "compatible", "ambiguous"]},
                        "severity": {"type": "string", "enum": ["info", "warning", "blocking"]},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                        "title": {"type": "string"},
                        "explanation": {"type": "string"}
                    }
                }
            }
        }
    })
}

#[derive(Default)]
pub struct DeterministicEngine;

#[async_trait]
impl AgentEngine for DeterministicEngine {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }

    fn requested_model(&self) -> &'static str {
        "deterministic-test-double"
    }

    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        Ok(deterministic_output(deterministic_turn(
            &input.scope_kind,
            &input.user_message,
            &input.context,
        )))
    }

    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        let selected_version_ids = input
            .candidates
            .as_array()
            .into_iter()
            .flatten()
            .filter(|candidate| {
                candidate.get("node_key").and_then(Value::as_str) == Some("product")
                    && candidate.get("entry_type").and_then(Value::as_str) == Some("decision")
            })
            .filter_map(|candidate| candidate.get("version_public_id").and_then(Value::as_str))
            .filter_map(|id| id.parse::<Uuid>().ok())
            .collect();
        Ok(deterministic_output(ContextSelectionDraft {
            selected_version_ids,
        }))
    }

    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        let requirements = input
            .context_pack
            .get("knowledge")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|item| item.get("entry_type").and_then(Value::as_str) == Some("requirement"))
            .collect::<Vec<_>>();
        let delivery_slices = if requirements.is_empty() {
            vec![TechnicalPlanSectionDraft {
                section_key: "implementation".into(),
                title: "Tranche de livraison".into(),
                body: format!(
                    "Décomposer et valider l'objectif sans supposer de code existant : {}",
                    input.objective
                ),
                source_version_ids: Vec::new(),
            }]
        } else {
            requirements
                .iter()
                .enumerate()
                .map(|(index, item)| TechnicalPlanSectionDraft {
                    section_key: format!("requirement-{}", index + 1),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("Exigence")
                        .into(),
                    body: item
                        .get("statement")
                        .and_then(Value::as_str)
                        .unwrap_or("Exigence à détailler")
                        .into(),
                    source_version_ids: item
                        .get("version_public_id")
                        .and_then(Value::as_str)
                        .and_then(|id| id.parse::<Uuid>().ok())
                        .into_iter()
                        .collect(),
                })
                .collect()
        };
        let coverage = requirements
            .iter()
            .filter_map(|item| {
                item.get("version_public_id")
                    .and_then(Value::as_str)
                    .and_then(|id| id.parse::<Uuid>().ok())
            })
            .map(|requirement_version_public_id| TechnicalPlanCoverageDraft {
                requirement_version_public_id,
                status: "missing".into(),
                explanation: "Aucune preuve externe valide n'est encore rattachée.".into(),
            })
            .collect();
        Ok(deterministic_output(TechnicalPlanDraft {
            title: "Technical Delivery Plan".into(),
            summary: format!("Plan de livraison pour : {}", input.objective),
            architecture: "Architecture à préciser dans l'outil de production externe à partir du ContextPack."
                .into(),
            delivery_slices,
            risks: vec!["Les preuves externes restent à importer et valider.".into()],
            validation: vec![
                "Relier chaque exigence à une preuve réelle et reproductible.".into(),
                "Valider le parcours web desktop et Tauri Linux.".into(),
            ],
            coverage,
        }))
    }

    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        let requirements = input
            .context_pack
            .get("knowledge")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|item| item.get("entry_type").and_then(Value::as_str) == Some("requirement"))
            .filter_map(|item| item.get("version_public_id").and_then(Value::as_str))
            .map(|id| {
                let requirement_version_public_id = id
                    .parse::<Uuid>()
                    .map_err(|_| AppError::Invalid("invalid requirement version in pack".into()))?;
                let section_keys = input
                    .technical_plan
                    .delivery_slices
                    .iter()
                    .filter(|section| {
                        section
                            .source_version_ids
                            .contains(&requirement_version_public_id)
                    })
                    .map(|section| section.section_key.clone())
                    .collect::<Vec<_>>();
                Ok(RequirementCoverageAssessment {
                    requirement_version_public_id,
                    source_version_ids: vec![requirement_version_public_id],
                    assessment: if section_keys.is_empty() {
                        "unaddressed"
                    } else {
                        "planned"
                    }
                    .into(),
                    section_keys,
                    explanation:
                        "Mapping du plan uniquement ; une preuve externe validée reste nécessaire."
                            .into(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(deterministic_output(CoverageEvaluationDraft {
            requirements,
        }))
    }

    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        Ok(deterministic_output(StewardOutput {
            assessments: deterministic_steward_assessments(&input.candidate_pairs)?,
        }))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RetentionPolicy {
    Indefinite,
    Finite,
    Unknown,
}

fn deterministic_steward_assessments(
    candidate_pairs: &Value,
) -> AppResult<Vec<ContradictionAssessmentDraft>> {
    let candidate_pairs = candidate_pairs
        .as_array()
        .ok_or_else(|| AppError::Agent("Steward candidate_pairs must be an array".into()))?;

    candidate_pairs
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let (left_id, left_text) = deterministic_candidate_side(candidate, "left", index)?;
            let (right_id, right_text) = deterministic_candidate_side(candidate, "right", index)?;
            if left_id == right_id {
                return Err(AppError::Agent(format!(
                    "Steward candidate pair {index} cites the same source twice"
                )));
            }

            let (verdict, severity, confidence, title, explanation) =
                deterministic_pair_classification(&left_text, &right_text);
            Ok(ContradictionAssessmentDraft {
                source_version_ids: vec![left_id, right_id],
                verdict: verdict.into(),
                severity: severity.into(),
                confidence,
                title: title.into(),
                explanation: explanation.into(),
            })
        })
        .collect()
}

fn deterministic_candidate_side(
    candidate: &Value,
    side_name: &str,
    candidate_index: usize,
) -> AppResult<(Uuid, String)> {
    let side = candidate.get(side_name).ok_or_else(|| {
        AppError::Agent(format!(
            "Steward candidate pair {candidate_index} has no {side_name} source"
        ))
    })?;
    let raw_id = side
        .get("version_public_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::Agent(format!(
                "Steward candidate pair {candidate_index} has no {side_name} source UUID"
            ))
        })?;
    let source_id = raw_id.parse::<Uuid>().map_err(|_| {
        AppError::Agent(format!(
            "Steward candidate pair {candidate_index} has an invalid {side_name} source UUID"
        ))
    })?;
    let text = ["title", "statement", "rationale"]
        .into_iter()
        .filter_map(|field| side.get(field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    Ok((source_id, text))
}

fn deterministic_pair_classification(
    left_text: &str,
    right_text: &str,
) -> (&'static str, &'static str, f64, &'static str, &'static str) {
    let left = left_text.to_lowercase().replace('’', "'");
    let right = right_text.to_lowercase().replace('’', "'");
    let left_policy = retention_policy(&left);
    let right_policy = retention_policy(&right);

    if matches!(
        (left_policy, right_policy),
        (RetentionPolicy::Indefinite, RetentionPolicy::Finite)
            | (RetentionPolicy::Finite, RetentionPolicy::Indefinite)
    ) {
        return (
            "contradiction",
            "blocking",
            0.96,
            "Contradiction de conservation",
            "Une source impose une conservation sans limite tandis que l'autre prévoit une expiration ou une suppression bornée.",
        );
    }

    let left_limits = temporal_limits(&left);
    let right_limits = temporal_limits(&right);
    let different_retention_limits = !left_limits.is_empty()
        && !right_limits.is_empty()
        && left_limits != right_limits
        && (left_policy == RetentionPolicy::Finite || right_policy == RetentionPolicy::Finite);
    let opposite_obligations = matches!(
        (deontic_polarity(&left), deontic_polarity(&right)),
        (-1, 1) | (1, -1)
    );

    if different_retention_limits || opposite_obligations {
        return (
            "ambiguous",
            "warning",
            0.68,
            "Portée à clarifier",
            "Les formulations présentent des bornes temporelles ou des obligations différentes; leur portée doit être précisée avant de conclure.",
        );
    }

    (
        "compatible",
        "info",
        0.82,
        "Connaissances compatibles",
        "Aucune incompatibilité déterministe directe n'est détectée; les deux formulations peuvent coexister.",
    )
}

fn retention_policy(text: &str) -> RetentionPolicy {
    let deletion_action = contains_any(
        text,
        &[
            "expir", "purge", "supprim", "effac", "delete", "delet", "remove", "retir",
        ],
    );
    let indefinite = contains_any(
        text,
        &[
            "n'expir",
            "ne expir",
            "sans expiration",
            "sans date d'expiration",
            "sans limite de temps",
            "indéfiniment",
            "indefinitely",
            "never expir",
            "never delet",
            "never remov",
            "toujours disponible",
            "always available",
            "en permanence",
            "permanently",
        ],
    ) || (text.contains("jamais") && deletion_action);
    if indefinite {
        return RetentionPolicy::Indefinite;
    }

    let retention_domain = deletion_action
        || contains_any(
            text,
            &[
                "conserv",
                "rétention",
                "retention",
                "retain",
                "stock",
                "archiv",
                "disponib",
            ],
        );
    let bounded_expression = contains_any(
        text,
        &[
            "après",
            "after",
            "au bout",
            "pendant",
            "durant",
            "jusqu'à",
            "jusqu’a",
            "for ",
            "maximum",
            "au plus",
        ],
    );
    if deletion_action
        || (retention_domain && bounded_expression && !temporal_limits(text).is_empty())
    {
        RetentionPolicy::Finite
    } else {
        RetentionPolicy::Unknown
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn temporal_limits(text: &str) -> Vec<u64> {
    let normalized = text
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let mut values = tokens
        .windows(2)
        .filter_map(|window| {
            let amount = window[0].parse::<u64>().ok()?;
            let multiplier = match window[1] {
                "minute" | "minutes" => 1,
                "heure" | "heures" | "hour" | "hours" => 60,
                "jour" | "jours" | "day" | "days" => 60 * 24,
                "semaine" | "semaines" | "week" | "weeks" => 60 * 24 * 7,
                "mois" | "month" | "months" => 60 * 24 * 30,
                "an" | "ans" | "année" | "années" | "year" | "years" => 60 * 24 * 365,
                _ => return None,
            };
            amount.checked_mul(multiplier)
        })
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

fn deontic_polarity(text: &str) -> i8 {
    if contains_any(
        text,
        &[
            "ne doit pas",
            "ne doit jamais",
            "ne peut pas",
            "interdit de",
            "must not",
            "shall not",
        ],
    ) {
        -1
    } else {
        i8::from(contains_any(
            text,
            &["doit", "doivent", "must", "shall", "required"],
        ))
    }
}

fn deterministic_output<T>(output: T) -> EngineOutput<T> {
    EngineOutput {
        output,
        metadata: AgentRunMetadata {
            provider: "deterministic".into(),
            requested_model: "deterministic-test-double".into(),
            served_model: Some("deterministic-test-double".into()),
            status: Some("completed".into()),
            estimated_cost: Some(0.0),
            attempts: 1,
            ..AgentRunMetadata::default()
        },
    }
}

fn deterministic_turn(scope: &str, message: &str, context: &Value) -> AgentTurn {
    let intent = message.trim();
    let intent = if intent.is_empty() {
        "La règle décrite par l’utilisateur doit être clarifiée."
    } else {
        intent
    };
    let sources = context
        .get("knowledge")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("version_public_id").and_then(Value::as_str))
        .filter_map(|id| id.parse::<Uuid>().ok())
        .collect();

    if scope == "product" {
        AgentTurn {
            response: "J’ai transformé cette intention en unités vérifiables. Vérifiez chaque proposition avant de l’inscrire au graphe.".into(),
            proposals: vec![
                proposal("business_rule", "Règle métier proposée", intent, "Cette formulation conserve l’intention utilisateur comme unité métier confirmable."),
                proposal("requirement", "Application observable de la règle", format!("Le système doit appliquer et rendre observable la règle confirmée : {intent}"), "Cette exigence rend la règle vérifiable sans inventer un domaine produit."),
                proposal("acceptance_criterion", "Vérification du comportement attendu", "Étant donné que la règle métier est confirmée, lorsque le comportement concerné est observé, alors le résultat respecte cette règle et conserve une preuve vérifiable.", "Le critère décrit une preuve générique à préciser pendant la validation humaine."),
            ],
            sources,
        }
    } else {
        AgentTurn {
            response: "Le contexte transmis suffit : je propose une décision technique atomique, sans reformuler le besoin Produit.".into(),
            proposals: vec![proposal(
                "technical_rule",
                "Décision technique proposée",
                intent,
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
        let turn = deterministic_turn(
            "product",
            "Les décisions validées restent consultables sans expiration.",
            &json!({}),
        );
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
        assert!(turn.proposals.iter().any(|item| {
            item.statement
                .contains("Les décisions validées restent consultables")
        }));
        assert!(
            turn.proposals
                .iter()
                .all(|item| !item.statement.to_lowercase().contains("crédit"))
        );
    }

    #[test]
    fn tech_turn_can_express_the_reference_conflict() {
        let turn = deterministic_turn(
            "tech",
            "Purger automatiquement les décisions après 90 jours.",
            &json!({}),
        );
        assert!(turn.proposals[0].statement.contains("90 jours"));
    }

    #[tokio::test]
    async fn deterministic_steward_assesses_every_pair_with_exact_source_ids() {
        let ids = (1..=6).map(Uuid::from_u128).collect::<Vec<_>>();
        let candidate_pairs = json!([
            steward_pair(
                ids[0],
                "Les documents validés sont conservés sans expiration.",
                ids[1],
                "Le système doit purger les documents après 90 jours."
            ),
            steward_pair(
                ids[2],
                "Chaque décision confirmée est versionnée.",
                ids[3],
                "Le journal d'audit référence les versions confirmées."
            ),
            steward_pair(
                ids[4],
                "Les journaux sont conservés pendant 30 jours.",
                ids[5],
                "La rétention des journaux dure pendant 90 jours."
            )
        ]);

        let generated = DeterministicEngine
            .analyze_contradictions(StewardInput { candidate_pairs })
            .await
            .expect("valid candidate pairs should be assessed");
        let assessments = generated.output.assessments;

        assert_eq!(assessments.len(), 3);
        assert_eq!(assessments[0].source_version_ids, vec![ids[0], ids[1]]);
        assert_eq!(assessments[0].verdict, "contradiction");
        assert_eq!(assessments[0].severity, "blocking");
        assert_eq!(assessments[1].source_version_ids, vec![ids[2], ids[3]]);
        assert_eq!(assessments[1].verdict, "compatible");
        assert_eq!(assessments[1].severity, "info");
        assert_eq!(assessments[2].source_version_ids, vec![ids[4], ids[5]]);
        assert_eq!(assessments[2].verdict, "ambiguous");
        assert_eq!(assessments[2].severity, "warning");
    }

    #[tokio::test]
    async fn deterministic_steward_rejects_a_pair_without_a_valid_source_uuid() {
        let result = DeterministicEngine
            .analyze_contradictions(StewardInput {
                candidate_pairs: json!([{
                    "left": {
                        "version_public_id": "not-a-uuid",
                        "statement": "Le contexte reste disponible."
                    },
                    "right": {
                        "version_public_id": Uuid::from_u128(2),
                        "statement": "Le contexte est archivé."
                    }
                }]),
            })
            .await;

        assert!(
            matches!(result, Err(AppError::Agent(message)) if message.contains("invalid left source UUID"))
        );
    }

    fn steward_pair(
        left_id: Uuid,
        left_statement: &str,
        right_id: Uuid,
        right_statement: &str,
    ) -> Value {
        json!({
            "left": {
                "version_public_id": left_id,
                "title": "Politique de contexte",
                "statement": left_statement,
                "rationale": ""
            },
            "right": {
                "version_public_id": right_id,
                "title": "Politique de contexte",
                "statement": right_statement,
                "rationale": ""
            }
        })
    }
}
