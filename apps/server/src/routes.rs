use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, Method, header},
    routing::{get, patch, post},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{
        CommitResult, CoverageView, CreateHandoff, CreateProject, CreateSession, DecideProposals,
        DeliverableSummary, GateResult, GenerateTechnicalPlan, HandoffView, Health, HistoryEvent,
        InsightAction, InsightDetail, InsightSummary, ProjectSnapshot, ProjectSummary,
        ReviseKnowledge, RevisionResult, SendMessage, SessionView,
    },
    service,
};

pub fn router(state: Arc<service::AppState>) -> Router {
    let origins = [
        HeaderValue::from_static("http://127.0.0.1:5173"),
        HeaderValue::from_static("http://localhost:5173"),
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("https://tauri.localhost"),
        HeaderValue::from_static("tauri://localhost"),
    ];
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/api/health", get(get_health))
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
            "/api/projects/{project_id}/deliverables/technical-plan",
            post(generate_technical_plan),
        )
        .route("/api/projects/{project_id}/coverage", get(project_coverage))
        .route("/api/projects/{project_id}/history", get(project_history))
        .route("/api/sessions/{session_id}", get(get_session))
        .route("/api/sessions/{session_id}/messages", post(send_message))
        .route(
            "/api/sessions/{session_id}/proposals/decision",
            post(decide_proposals),
        )
        .route("/api/knowledge/{knowledge_id}", patch(revise_knowledge))
        .route("/api/insights", get(list_insights))
        .route(
            "/api/insights/{insight_id}",
            get(get_insight).patch(act_on_insight),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn get_health(State(state): State<Arc<service::AppState>>) -> AppResult<Json<Health>> {
    Ok(Json(service::health(&state).await?))
}

async fn list_projects(
    State(state): State<Arc<service::AppState>>,
) -> AppResult<Json<Vec<ProjectSummary>>> {
    Ok(Json(service::list_projects(&state).await?))
}

async fn create_project(
    State(state): State<Arc<service::AppState>>,
    Json(input): Json<CreateProject>,
) -> AppResult<Json<ProjectSummary>> {
    Ok(Json(service::create_project(&state, input).await?))
}

async fn project_snapshot(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<ProjectSnapshot>> {
    Ok(Json(service::snapshot(&state, project_id).await?))
}

async fn create_session(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateSession>,
) -> AppResult<Json<SessionView>> {
    Ok(Json(
        service::create_session(&state, project_id, input).await?,
    ))
}

async fn get_session(
    State(state): State<Arc<service::AppState>>,
    Path(session_id): Path<Uuid>,
) -> AppResult<Json<SessionView>> {
    Ok(Json(service::session_view(&state, session_id).await?))
}

async fn send_message(
    State(state): State<Arc<service::AppState>>,
    Path(session_id): Path<Uuid>,
    Json(input): Json<SendMessage>,
) -> AppResult<Json<SessionView>> {
    Ok(Json(
        service::send_message(&state, session_id, input).await?,
    ))
}

async fn decide_proposals(
    State(state): State<Arc<service::AppState>>,
    Path(session_id): Path<Uuid>,
    Json(input): Json<DecideProposals>,
) -> AppResult<Json<CommitResult>> {
    Ok(Json(
        service::decide_proposals(&state, session_id, input).await?,
    ))
}

async fn evaluate_gate(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<GateResult>> {
    Ok(Json(
        service::evaluate_product_gate(&state, project_id).await?,
    ))
}

async fn generate_feature_brief(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<DeliverableSummary>> {
    Ok(Json(
        service::generate_feature_brief(&state, project_id).await?,
    ))
}

async fn create_handoff(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateHandoff>,
) -> AppResult<Json<HandoffView>> {
    Ok(Json(
        service::create_handoff(&state, project_id, input).await?,
    ))
}

async fn generate_technical_plan(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<GenerateTechnicalPlan>,
) -> AppResult<Json<DeliverableSummary>> {
    Ok(Json(
        service::generate_technical_plan(&state, project_id, input).await?,
    ))
}

async fn project_coverage(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<CoverageView>> {
    Ok(Json(service::coverage(&state, project_id).await?))
}

async fn project_history(
    State(state): State<Arc<service::AppState>>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<HistoryEvent>>> {
    Ok(Json(service::project_history(&state, project_id).await?))
}

async fn revise_knowledge(
    State(state): State<Arc<service::AppState>>,
    Path(knowledge_id): Path<Uuid>,
    Json(input): Json<ReviseKnowledge>,
) -> AppResult<Json<RevisionResult>> {
    Ok(Json(
        service::revise_knowledge(&state, knowledge_id, input).await?,
    ))
}

async fn list_insights(
    State(state): State<Arc<service::AppState>>,
) -> AppResult<Json<Vec<InsightSummary>>> {
    Ok(Json(service::list_insights(&state).await?))
}

async fn get_insight(
    State(state): State<Arc<service::AppState>>,
    Path(insight_id): Path<Uuid>,
) -> AppResult<Json<InsightDetail>> {
    Ok(Json(service::insight_detail(&state, insight_id).await?))
}

async fn act_on_insight(
    State(state): State<Arc<service::AppState>>,
    Path(insight_id): Path<Uuid>,
    Json(input): Json<InsightAction>,
) -> AppResult<Json<InsightDetail>> {
    Ok(Json(
        service::act_on_insight(&state, insight_id, input).await?,
    ))
}
