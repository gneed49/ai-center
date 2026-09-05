use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::{
    auth::{AuthRuntime, RequestContext},
    error::{AppError, AppResult},
    external_references::{
        self, CreateExternalEvidence, CreateGitHubReference, ExternalEvidenceView,
        ExternalReferenceSummary, ExternalReferenceView, ReviewExternalEvidence,
    },
    idempotency::{self, BeginOutcome, BeginRequest, FailureDisposition, IdempotencyLease},
    integrations::GitHubRuntime,
    models::{
        CompileContextPack, ContextPackSummary, CoverageView, CreateHandoff, CreateProject,
        CreateSession, DecideProposals, GenerateTechnicalPlan, HandoffView, Health, HistoryEvent,
        InsightAction, InsightDetail, InsightSummary, ProjectSnapshot, ProjectSummary,
        ResolveInsight, ReviseKnowledge, SendMessage, SessionView, WorkspaceSummary,
    },
    service,
};

#[derive(Clone)]
struct ApiState {
    service: Arc<service::AppState>,
    auth: Arc<AuthRuntime>,
    github: Option<Arc<GitHubRuntime>>,
}

pub fn router(
    state: Arc<service::AppState>,
    auth: Arc<AuthRuntime>,
    github: Option<Arc<GitHubRuntime>>,
    origins: Vec<HeaderValue>,
) -> Router {
    let state = ApiState {
        service: state,
        auth,
        github,
    };
    let request_id_header = header::HeaderName::from_static("x-request-id");
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-ai-center-workspace-id"),
            header::HeaderName::from_static("idempotency-key"),
        ])
        .expose_headers([request_id_header.clone()]);

    let protected = Router::new()
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{project_id}/snapshot", get(project_snapshot))
        .route("/api/projects/{project_id}/sessions", post(create_session))
        .route(
            "/api/projects/{project_id}/gates/product-ready/evaluate",
            post(evaluate_gate),
        )
        .route(
            "/api/projects/{project_id}/deliverables/feature-brief",
            post(generate_feature_brief),
        )
        .route("/api/projects/{project_id}/handoffs", post(create_handoff))
        .route(
            "/api/projects/{project_id}/handoffs/latest",
            get(latest_handoff),
        )
        .route(
            "/api/projects/{project_id}/context-packs",
            post(compile_context_pack),
        )
        .route(
            "/api/context-packs/{context_pack_id}",
            get(get_context_pack),
        )
        .route(
            "/api/context-packs/{context_pack_id}/export",
            get(export_context_pack),
        )
        .route(
            "/api/projects/{project_id}/deliverables/technical-plan",
            post(generate_technical_plan),
        )
        .route("/api/projects/{project_id}/coverage", get(project_coverage))
        .route("/api/projects/{project_id}/history", get(project_history))
        .merge(external_reference_routes())
        .route(
            "/api/projects/{project_id}/sessions/{session_id}",
            get(get_session),
        )
        .route(
            "/api/projects/{project_id}/sessions/{session_id}/messages",
            post(send_message),
        )
        .route(
            "/api/projects/{project_id}/sessions/{session_id}/proposals/decision",
            post(decide_proposals),
        )
        .route(
            "/api/projects/{project_id}/knowledge/{knowledge_id}",
            patch(revise_knowledge),
        )
        .route("/api/insights", get(list_insights))
        .route("/api/insights/{insight_id}", get(get_insight))
        .route(
            "/api/projects/{project_id}/insights/{insight_id}",
            get(get_project_insight).patch(act_on_project_insight),
        )
        .route(
            "/api/projects/{project_id}/insights/{insight_id}/resolve",
            post(resolve_insight),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_context,
        ));

    Router::new()
        .route("/api/health", get(get_health))
        .merge(protected)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(cors)
        .with_state(state)
}

fn external_reference_routes() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/projects/{project_id}/external-references",
            get(list_external_references).post(create_external_reference),
        )
        .route(
            "/api/external-references/{reference_id}",
            get(get_external_reference),
        )
        .route(
            "/api/external-references/{reference_id}/refresh",
            post(refresh_external_reference),
        )
        .route(
            "/api/external-references/{reference_id}/evidence",
            post(create_external_evidence),
        )
        .route(
            "/api/external-references/{reference_id}/evidence/{evidence_id}/review",
            post(review_external_evidence),
        )
}

async fn require_context(
    State(state): State<ApiState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let context = if request.uri().path() == "/api/workspaces" {
        RequestContext {
            actor_id: state.auth.authenticate_actor(&headers).await?,
            workspace_id: Uuid::nil(),
            workspace_internal_id: None,
            workspace_role: "viewer".into(),
        }
    } else {
        state
            .auth
            .authenticate(&headers, &state.service.pool)
            .await?
    };
    if matches!(request.method(), &Method::POST | &Method::PATCH) {
        if context.workspace_role == "viewer" {
            return Err(crate::error::AppError::Forbidden);
        }
        let idempotency_key = headers
            .get("idempotency-key")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Uuid>().ok())
            .ok_or_else(|| {
                crate::error::AppError::Invalid(
                    "Idempotency-Key must be a client-generated UUID".into(),
                )
            })?;
        request.extensions_mut().insert(idempotency_key);
    }
    request.extensions_mut().insert(context);
    Ok(next.run(request).await)
}

fn scoped(state: &ApiState, context: &RequestContext) -> service::AppState {
    state.service.scoped(context)
}

fn github_runtime(state: &ApiState) -> AppResult<Arc<GitHubRuntime>> {
    state
        .github
        .clone()
        .ok_or_else(|| AppError::Connector("GitHub App is not configured for this server".into()))
}

async fn set_request_context(
    tx: &mut Transaction<'_, Postgres>,
    context: &RequestContext,
) -> AppResult<i64> {
    let workspace_internal_id = context.workspace_internal_id.ok_or_else(|| {
        crate::error::AppError::Internal("workspace internal id is missing".into())
    })?;
    if workspace_internal_id <= 0 {
        return Err(crate::error::AppError::Internal(
            "workspace internal id is invalid".into(),
        ));
    }
    if !matches!(context.workspace_role.as_str(), "owner" | "editor") {
        return Err(crate::error::AppError::Forbidden);
    }
    sqlx::query(
        "select set_config('app.current_actor_id', $1, true),
                set_config('app.current_workspace_id', $2, true),
                set_config('app.current_workspace_role', $3, true)",
    )
    .bind(context.actor_id.to_string())
    .bind(workspace_internal_id.to_string())
    .bind(&context.workspace_role)
    .execute(&mut **tx)
    .await?;
    Ok(workspace_internal_id)
}

async fn begin_idempotent<T: Serialize + ?Sized>(
    state: &ApiState,
    context: &RequestContext,
    project_public_id: Option<Uuid>,
    operation: &str,
    key: Uuid,
    request: &T,
) -> AppResult<BeginOutcome> {
    let mut tx = state.service.pool.begin().await?;
    let workspace_internal_id = set_request_context(&mut tx, context).await?;
    let project_id = if let Some(project_public_id) = project_public_id {
        Some(
            sqlx::query_scalar(
                "select id from app.projects where public_id = $1 and workspace_id = $2",
            )
            .bind(project_public_id)
            .bind(workspace_internal_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(crate::error::AppError::NotFound)?,
        )
    } else {
        None
    };
    let request_hash = idempotency::hash_request(request)?;
    let key = key.to_string();
    let outcome = idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: workspace_internal_id,
            project_id,
            actor_id: context.actor_id,
            operation_key: operation,
            idempotency_key: &key,
            request_hash: &request_hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(outcome)
}

async fn finish_idempotent(
    state: &ApiState,
    context: &RequestContext,
    lease: &IdempotencyLease,
    result: &Result<Value, crate::error::AppError>,
) -> AppResult<idempotency::StoredResponse> {
    let mut tx = state.service.pool.begin().await?;
    let workspace_internal_id = set_request_context(&mut tx, context).await?;
    if lease.workspace_id != workspace_internal_id || lease.actor_id != context.actor_id {
        return Err(crate::error::AppError::Internal(
            "idempotency lease is outside the authenticated request scope".into(),
        ));
    }
    let response = match result {
        Ok(body) => {
            idempotency::complete(&mut tx, lease, StatusCode::OK.as_u16(), body.clone()).await?
        }
        Err(error) => {
            idempotency::fail(
                &mut tx,
                lease,
                error.status_code().as_u16(),
                error.public_code(),
                failure_disposition(error),
                json!({"code": error.public_code(), "message": error.public_message()}),
            )
            .await?
        }
    };
    tx.commit().await?;
    Ok(response)
}

fn failure_disposition(error: &AppError) -> FailureDisposition {
    match error {
        // AI-provider retries are driven only by the typed outcome of its
        // bounded local retry loop. Agent/output validation is permanent.
        AppError::Provider(_) if error.is_retryable_provider_failure() => {
            FailureDisposition::Retryable
        }
        // Connector classification remains connector-owned.
        AppError::Connector(_) | AppError::ConnectorRateLimited { .. } => {
            FailureDisposition::Retryable
        }
        AppError::Unauthorized
        | AppError::Forbidden
        | AppError::NotFound
        | AppError::Invalid(_)
        | AppError::Conflict(_)
        | AppError::Agent(_)
        | AppError::Provider(_)
        | AppError::Database(_)
        | AppError::Internal(_) => FailureDisposition::Permanent,
    }
}

#[cfg(test)]
mod provider_failure_tests {
    use super::failure_disposition;
    use crate::{
        error::{AppError, ProviderError, ProviderErrorClass},
        idempotency::FailureDisposition,
    };
    use axum::http::StatusCode;

    #[test]
    fn only_explicit_transient_provider_classes_resume_durable_commands() {
        for class in [
            ProviderErrorClass::Timeout,
            ProviderErrorClass::Connection,
            ProviderErrorClass::RateLimit,
            ProviderErrorClass::Server,
        ] {
            let error = AppError::Provider(ProviderError::new("OpenAI", class, 3, None));
            assert_eq!(failure_disposition(&error), FailureDisposition::Retryable);
            assert_eq!(error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        }

        for class in [
            ProviderErrorClass::Authentication,
            ProviderErrorClass::Permission,
            ProviderErrorClass::NotFound,
            ProviderErrorClass::Request,
            ProviderErrorClass::Quota,
            ProviderErrorClass::ResponseContract,
            ProviderErrorClass::Transport,
        ] {
            let error = AppError::Provider(ProviderError::new("OpenAI", class, 1, None));
            assert_eq!(failure_disposition(&error), FailureDisposition::Permanent);
            assert_eq!(error.status_code(), StatusCode::BAD_GATEWAY);
        }

        assert_eq!(
            failure_disposition(&AppError::Agent("invalid structured output".into())),
            FailureDisposition::Permanent
        );
    }
}

enum MutationStart {
    Execute(IdempotencyLease),
    Respond(Response),
}

fn mutation_start(outcome: BeginOutcome) -> AppResult<MutationStart> {
    match outcome {
        BeginOutcome::New { lease, .. } => Ok(MutationStart::Execute(lease)),
        BeginOutcome::InProgress { locked_until, .. } => Err(crate::error::AppError::Conflict(
            format!("operation is still processing until {locked_until}"),
        )),
        BeginOutcome::Replay { response, .. } => {
            let status = StatusCode::from_u16(response.status_code)
                .map_err(|_| crate::error::AppError::Internal("stored status is invalid".into()))?;
            Ok(MutationStart::Respond(
                (status, Json(response.body)).into_response(),
            ))
        }
    }
}

async fn finalize_mutation<T: Serialize>(
    state: &ApiState,
    context: &RequestContext,
    lease: &IdempotencyLease,
    result: AppResult<T>,
) -> AppResult<Response> {
    let recorded = match result {
        Ok(output) => serde_json::to_value(output)
            .map_err(|error| crate::error::AppError::Internal(error.to_string())),
        Err(error) => Err(error),
    };
    let response = finish_idempotent(state, context, lease, &recorded).await?;
    let status = StatusCode::from_u16(response.status_code)
        .map_err(|_| crate::error::AppError::Internal("stored status is invalid".into()))?;
    Ok((status, Json(response.body)).into_response())
}

async fn get_health(State(state): State<ApiState>) -> AppResult<Json<Health>> {
    Ok(Json(service::health(&state.service).await?))
}

async fn list_projects(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<ProjectSummary>>> {
    Ok(Json(
        service::list_projects(&scoped(&state, &context)).await?,
    ))
}

async fn list_workspaces(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<WorkspaceSummary>>> {
    Ok(Json(
        service::list_workspaces(&scoped(&state, &context)).await?,
    ))
}

async fn create_project(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Json(input): Json<CreateProject>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        None,
        "project.create",
        idempotency_key,
        &input,
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result =
                service::create_project_idempotent(&scoped(&state, &context), input, &lease).await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn project_snapshot(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<ProjectSnapshot>> {
    Ok(Json(
        service::snapshot(&scoped(&state, &context), project_id).await?,
    ))
}

async fn create_session(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateSession>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "session.create",
        idempotency_key,
        &input,
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::create_session_idempotent(
                &scoped(&state, &context),
                project_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn get_session(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((project_id, session_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<SessionView>> {
    Ok(Json(
        service::session_view_for_project(&scoped(&state, &context), project_id, session_id)
            .await?,
    ))
}

async fn send_message(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((project_id, session_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<SendMessage>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "session.send_message",
        idempotency_key,
        &(session_id, &input),
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::send_message_for_project_idempotent(
                &scoped(&state, &context),
                project_id,
                session_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn decide_proposals(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((project_id, session_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<DecideProposals>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "proposal.decide",
        idempotency_key,
        &(session_id, &input),
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::decide_proposals_for_project_idempotent(
                &scoped(&state, &context),
                project_id,
                session_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn evaluate_gate(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "gate.product_ready.evaluate",
        idempotency_key,
        &"product-ready",
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::evaluate_product_gate_idempotent(
                &scoped(&state, &context),
                project_id,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn generate_feature_brief(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "deliverable.feature_brief.generate",
        idempotency_key,
        &"feature-brief",
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::generate_feature_brief_idempotent(
                &scoped(&state, &context),
                project_id,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn create_handoff(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateHandoff>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "handoff.create",
        idempotency_key,
        &input,
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::create_handoff_idempotent(
                &scoped(&state, &context),
                project_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn compile_context_pack(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CompileContextPack>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "context_pack.compile",
        idempotency_key,
        &input,
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::compile_context_pack_idempotent(
                &scoped(&state, &context),
                project_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn latest_handoff(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Option<HandoffView>>> {
    Ok(Json(
        service::latest_handoff(&scoped(&state, &context), project_id).await?,
    ))
}

async fn get_context_pack(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(context_pack_id): Path<Uuid>,
) -> AppResult<Json<ContextPackSummary>> {
    Ok(Json(
        service::context_pack(&scoped(&state, &context), context_pack_id).await?,
    ))
}

#[derive(Deserialize)]
struct ContextPackExportQuery {
    format: String,
}

async fn export_context_pack(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(context_pack_id): Path<Uuid>,
    Query(query): Query<ContextPackExportQuery>,
) -> AppResult<Response> {
    let (content_type, body) =
        service::export_context_pack(&scoped(&state, &context), context_pack_id, &query.format)
            .await?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=context-pack"),
            ),
        ],
        body,
    )
        .into_response())
}

async fn generate_technical_plan(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<GenerateTechnicalPlan>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "technical_plan.generate",
        idempotency_key,
        &input,
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::generate_technical_plan_idempotent(
                &scoped(&state, &context),
                project_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn project_coverage(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<CoverageView>> {
    Ok(Json(
        service::coverage(&scoped(&state, &context), project_id).await?,
    ))
}

async fn project_history(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<HistoryEvent>>> {
    Ok(Json(
        service::project_history(&scoped(&state, &context), project_id).await?,
    ))
}

async fn list_external_references(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<ExternalReferenceSummary>>> {
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::list(&service, &context, project_id).await?,
    ))
}

async fn create_external_reference(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateGitHubReference>,
) -> AppResult<Json<ExternalReferenceView>> {
    let github = github_runtime(&state)?;
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::create_github_reference(
            &service,
            &context,
            &github,
            project_id,
            idempotency_key,
            input,
        )
        .await?,
    ))
}

async fn get_external_reference(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(reference_id): Path<Uuid>,
) -> AppResult<Json<ExternalReferenceView>> {
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::get(&service, &context, reference_id).await?,
    ))
}

async fn refresh_external_reference(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(reference_id): Path<Uuid>,
) -> AppResult<Json<ExternalReferenceView>> {
    let github = github_runtime(&state)?;
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::refresh(&service, &context, &github, reference_id, idempotency_key)
            .await?,
    ))
}

async fn create_external_evidence(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path(reference_id): Path<Uuid>,
    Json(input): Json<CreateExternalEvidence>,
) -> AppResult<Json<ExternalEvidenceView>> {
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::create_evidence(
            &service,
            &context,
            reference_id,
            idempotency_key,
            input,
        )
        .await?,
    ))
}

async fn review_external_evidence(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((reference_id, evidence_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ReviewExternalEvidence>,
) -> AppResult<Json<ExternalEvidenceView>> {
    let service = scoped(&state, &context);
    Ok(Json(
        external_references::review_evidence(
            &service,
            &context,
            reference_id,
            evidence_id,
            idempotency_key,
            input,
        )
        .await?,
    ))
}

async fn revise_knowledge(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((project_id, knowledge_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ReviseKnowledge>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "knowledge.revise",
        idempotency_key,
        &(knowledge_id, &input),
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::revise_knowledge_for_project_idempotent(
                &scoped(&state, &context),
                project_id,
                knowledge_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn list_insights(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<InsightSummary>>> {
    Ok(Json(
        service::list_insights(&scoped(&state, &context)).await?,
    ))
}

async fn get_insight(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(insight_id): Path<Uuid>,
) -> AppResult<Json<InsightDetail>> {
    Ok(Json(
        service::insight_detail(&scoped(&state, &context), insight_id).await?,
    ))
}

async fn get_project_insight(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((project_id, insight_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<InsightDetail>> {
    Ok(Json(
        service::insight_detail_for_project(&scoped(&state, &context), project_id, insight_id)
            .await?,
    ))
}

async fn act_on_project_insight(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((project_id, insight_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<InsightAction>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "insight.review",
        idempotency_key,
        &(insight_id, &input),
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::act_on_insight_for_project_idempotent(
                &scoped(&state, &context),
                project_id,
                insight_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}

async fn resolve_insight(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(idempotency_key): Extension<Uuid>,
    Path((project_id, insight_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ResolveInsight>,
) -> AppResult<Response> {
    let outcome = begin_idempotent(
        &state,
        &context,
        Some(project_id),
        "insight.resolve",
        idempotency_key,
        &(insight_id, &input),
    )
    .await?;
    match mutation_start(outcome)? {
        MutationStart::Respond(response) => Ok(response),
        MutationStart::Execute(lease) => {
            let result = service::resolve_insight_idempotent(
                &scoped(&state, &context),
                project_id,
                insight_id,
                input,
                &lease,
            )
            .await;
            finalize_mutation(&state, &context, &lease, result).await
        }
    }
}
