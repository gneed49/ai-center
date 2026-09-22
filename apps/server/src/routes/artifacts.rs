//! Artifact commands share authenticated request context and durable leases.
use super::{
    ApiState, AppError, AppResult, Extension, Json, MutationStart, Path, Query, RequestContext,
    Response, Router, State, Uuid, begin_idempotent, finalize_mutation, get, mutation_start, post,
    scoped,
};
use axum::{
    http::{HeaderValue, header},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifacts::{
    self, ArtifactDetail, ArtifactSummary, ArtifactVersion, CreateArtifact, Destinations,
    ListArtifacts, Page, ResetDestination, SaveDraft, SetDestination, ValidateArtifact,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/projects/{project_id}/artifacts",
            get(list).post(create),
        )
        .route("/api/artifacts/{id}", get(detail))
        .route("/api/artifacts/{id}/draft", post(save))
        .route("/api/artifacts/{id}/validate", post(validate))
        .route("/api/artifacts/{id}/versions", get(versions))
        .route("/api/artifacts/{id}/versions/{version_id}", get(version))
        .route(
            "/api/artifacts/{id}/versions/{version_id}/export",
            get(export),
        )
        .route(
            "/api/artifact-destinations",
            get(company_destinations).post(set_company_destination),
        )
        .route(
            "/api/artifact-destinations/reset",
            post(reset_company_destination),
        )
        .route(
            "/api/projects/{project_id}/artifact-destinations",
            get(project_destinations).post(set_project_destination),
        )
        .route(
            "/api/projects/{project_id}/artifact-destinations/reset",
            post(reset_project_destination),
        )
}

async fn list(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(project): Path<Uuid>,
    Query(query): Query<ListArtifacts>,
) -> AppResult<Json<Page<ArtifactSummary>>> {
    Ok(Json(
        artifacts::list(&scoped(&state, &context), project, query).await?,
    ))
}
async fn detail(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ArtifactDetail>> {
    Ok(Json(artifacts::get(&scoped(&state, &context), id).await?))
}
async fn versions(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListArtifacts>,
) -> AppResult<Json<Page<ArtifactVersion>>> {
    Ok(Json(
        artifacts::versions(&scoped(&state, &context), id, query).await?,
    ))
}
async fn version(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((id, version)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ArtifactVersion>> {
    Ok(Json(
        artifacts::version(&scoped(&state, &context), id, version).await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportQuery {
    format: Option<String>,
}
async fn export(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((id, version)): Path<(Uuid, Uuid)>,
    Query(query): Query<ExportQuery>,
) -> AppResult<Response> {
    let format = query.format.as_deref().unwrap_or("markdown");
    if !matches!(format, "markdown" | "json") {
        return Err(AppError::Invalid(
            "Export format must be markdown or json".into(),
        ));
    }
    let version = artifacts::version(&scoped(&state, &context), id, version).await?;
    let (body, content_type, extension) = if format == "json" {
        (
            serde_json::to_string_pretty(&json!({"artifact_id":id,"version":version}))
                .map_err(|error| AppError::Internal(error.to_string()))?,
            "application/json; charset=utf-8",
            "json",
        )
    } else {
        (
            artifacts::markdown(id, &version),
            "text/markdown; charset=utf-8",
            "md",
        )
    };
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"artifact-{id}-v{}.{}\"",
            version.version, extension
        ))
        .map_err(|_| AppError::Internal("Invalid export filename".into()))?,
    );
    Ok(response)
}

enum Command {
    Create(Uuid, CreateArtifact),
    Save(Uuid, SaveDraft),
    Validate(Uuid, ValidateArtifact),
    Destination(Option<Uuid>, SetDestination),
    Reset(Option<Uuid>, ResetDestination),
}
impl Command {
    fn request(&self) -> Value {
        match self {
            Self::Create(id, input) => json!({"project_id":id,"input":input}),
            Self::Save(id, input) => json!({"artifact_id":id,"input":input}),
            Self::Validate(id, input) => json!({"artifact_id":id,"input":input}),
            Self::Destination(id, input) => json!({"project_id":id,"input":input}),
            Self::Reset(id, input) => json!({"project_id":id,"input":input}),
        }
    }
    const fn operation(&self) -> &'static str {
        match self {
            Self::Create(..) => "artifact.create",
            Self::Save(..) => "artifact.save_draft",
            Self::Validate(..) => "artifact.validate",
            Self::Destination(..) => "artifact.destination.set",
            Self::Reset(..) => "artifact.destination.reset",
        }
    }
}
async fn command(
    state: ApiState,
    context: RequestContext,
    key: Uuid,
    command: Command,
) -> AppResult<Response> {
    let lease = match mutation_start(
        begin_idempotent(
            &state,
            &context,
            None,
            command.operation(),
            key,
            &command.request(),
        )
        .await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let scoped = scoped(&state, &context);
    let result: AppResult<Value> = match command {
        Command::Create(id, input) => artifacts::create(&scoped, id, input, Some(&lease))
            .await
            .map(|v| json!(v)),
        Command::Save(id, input) => artifacts::save_draft(&scoped, id, input, Some(&lease))
            .await
            .map(|v| json!(v)),
        Command::Validate(id, input) => artifacts::validate(&scoped, id, input, Some(&lease))
            .await
            .map(|v| json!(v)),
        Command::Destination(id, input) => {
            artifacts::set_destination(&scoped, id, input, Some(&lease))
                .await
                .map(|v| json!(v))
        }
        Command::Reset(id, input) => artifacts::reset_destination(&scoped, id, input, Some(&lease))
            .await
            .map(|v| json!(v)),
    };
    finalize_mutation(&state, &context, &lease, result).await
}
async fn create(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateArtifact>,
) -> AppResult<Response> {
    command(state, context, key, Command::Create(id, input)).await
}
async fn save(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<SaveDraft>,
) -> AppResult<Response> {
    command(state, context, key, Command::Save(id, input)).await
}
async fn validate(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<ValidateArtifact>,
) -> AppResult<Response> {
    command(state, context, key, Command::Validate(id, input)).await
}
async fn company_destinations(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Destinations>> {
    Ok(Json(
        artifacts::destinations(&scoped(&state, &context), None).await?,
    ))
}
async fn project_destinations(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Destinations>> {
    Ok(Json(
        artifacts::destinations(&scoped(&state, &context), Some(id)).await?,
    ))
}
async fn set_company_destination(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<SetDestination>,
) -> AppResult<Response> {
    command(state, context, key, Command::Destination(None, input)).await
}
async fn set_project_destination(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<SetDestination>,
) -> AppResult<Response> {
    command(state, context, key, Command::Destination(Some(id), input)).await
}
async fn reset_company_destination(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<ResetDestination>,
) -> AppResult<Response> {
    command(state, context, key, Command::Reset(None, input)).await
}
async fn reset_project_destination(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<ResetDestination>,
) -> AppResult<Response> {
    command(state, context, key, Command::Reset(Some(id), input)).await
}
