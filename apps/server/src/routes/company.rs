//! Company routes share authentication and atomic command completion with the
//! existing API. Only bootstrap operates before a workspace has been selected.
use super::{
    ApiState, AppError, AppResult, Extension, IdempotencyLease, IntoResponse, Json, MutationStart,
    Path, Query, RequestContext, Response, Router, Serialize, State, StatusCode, Uuid, Value,
    WorkspaceSummary, begin_idempotent, finish_idempotent, get, mutation_start, patch, post,
    scoped, service,
};
use crate::company::{
    self,
    models::{
        CompanyInput, CompanyMember, CompanyOverview, CreateCompany, CreateGraphEdge, GraphQuery,
        GraphView, UpdateMember,
    },
};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/workspaces/capabilities", get(creation_capabilities))
        .route("/api/company", get(overview).patch(update))
        .route("/api/company/export", get(export_data))
        .route("/api/company/projects/archived", get(archived_projects))
        .route("/api/projects/{id}/archive", post(archive_project))
        .route("/api/projects/{id}/export", get(export_project))
        .route("/api/projects/{id}", patch(rename_project))
        .route("/api/company/setup", post(setup))
        .route("/api/company/members", get(members))
        .route("/api/company/members/{id}", patch(update_member))
        .route("/api/company/graph", get(company_graph))
        .route("/api/projects/{id}/graph", get(project_graph))
        .route(
            "/api/projects/{project}/sources/{kind}/{id}",
            get(graph_source),
        )
        .route("/api/graph/edges", post(create_edge))
}

pub(super) async fn create_workspace(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<CreateCompany>,
) -> AppResult<Json<WorkspaceSummary>> {
    if !state
        .auth
        .may_create_company(context.actor_id, &state.service.pool)
        .await?
    {
        return Err(AppError::CompanyCreationNotAllowed);
    }
    let result = company::create(&scoped(&state, &context), input).await?;
    Ok(Json(WorkspaceSummary {
        public_id: result.workspace.public_id,
        name: result.workspace.name,
        role: result.workspace.role,
    }))
}

async fn creation_capabilities(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Value>> {
    Ok(Json(serde_json::json!({"can_create_company":
        state.auth.may_create_company(context.actor_id, &state.service.pool).await?
    })))
}

async fn overview(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<CompanyOverview>> {
    Ok(Json(company::overview(&scoped(&state, &context)).await?))
}

async fn members(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<CompanyMember>>> {
    Ok(Json(company::members(&scoped(&state, &context)).await?))
}

async fn company_graph(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Query(query): Query<GraphQuery>,
) -> AppResult<Json<GraphView>> {
    Ok(Json(
        company::graph(&scoped(&state, &context), query).await?,
    ))
}

async fn project_graph(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(mut query): Query<GraphQuery>,
) -> AppResult<Json<GraphView>> {
    if query.project_id.is_some_and(|project| project != id) {
        return Err(AppError::Invalid("Conflicting graph project scopes".into()));
    }
    query.project_id = Some(id);
    Ok(Json(
        company::graph(&scoped(&state, &context), query).await?,
    ))
}

async fn graph_source(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((project, kind, id)): Path<(Uuid, String, Uuid)>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        company::graph_source::read(&scoped(&state, &context), project, &kind, id).await?,
    ))
}

#[derive(Serialize)]
enum Command {
    Setup(CompanyInput),
    Update(CompanyInput),
    Member(Uuid, UpdateMember),
    Edge(CreateGraphEdge),
    Archive(Uuid, company::data::ArchiveProject),
    Rename(Uuid, company::data::RenameProject),
}

impl Command {
    fn operation(&self) -> &'static str {
        match self {
            Self::Setup(_) => "company.setup",
            Self::Update(_) => "company.update",
            Self::Member(_, _) => "company.member.update",
            Self::Edge(_) => "company.graph.edge.create",
            Self::Archive(_, _) => "company.project.archive",
            Self::Rename(_, _) => "company.project.rename",
        }
    }
    async fn execute(
        self,
        state: &service::AppState,
        lease: &IdempotencyLease,
    ) -> AppResult<Value> {
        let result = match self {
            Self::Setup(input) => {
                serde_json::to_value(company::update(state, input, true, Some(lease)).await?)
            }
            Self::Update(input) => {
                serde_json::to_value(company::update(state, input, false, Some(lease)).await?)
            }
            Self::Member(id, input) => {
                serde_json::to_value(company::update_member(state, id, input, Some(lease)).await?)
            }
            Self::Edge(input) => {
                serde_json::to_value(company::create_edge(state, input, Some(lease)).await?)
            }
            Self::Archive(id, input) => {
                serde_json::to_value(company::data::archive(state, id, input, Some(lease)).await?)
            }
            Self::Rename(id, input) => {
                serde_json::to_value(company::data::rename(state, id, input, Some(lease)).await?)
            }
        };
        result.map_err(|error| AppError::Internal(error.to_string()))
    }
}

async fn mutate(
    state: ApiState,
    context: RequestContext,
    key: Uuid,
    command: Command,
) -> AppResult<Response> {
    // Refuse owner-only mutations before acquiring a command lease.
    if !matches!(&command, Command::Edge(_) | Command::Rename(_, _))
        && context.workspace_role != "owner"
    {
        return Err(AppError::Forbidden);
    }
    let lease = match mutation_start(
        begin_idempotent(&state, &context, None, command.operation(), key, &command).await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let result = command.execute(&scoped(&state, &context), &lease).await;
    match result {
        Ok(body) => Ok(Json(body).into_response()),
        Err(error) => finish_idempotent(&state, &context, &lease, &Err(error))
            .await
            .map(|stored| {
                (
                    StatusCode::from_u16(stored.status_code)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    Json(stored.body),
                )
                    .into_response()
            }),
    }
}

async fn setup(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<CompanyInput>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Setup(input)).await
}
async fn update(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<CompanyInput>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Update(input)).await
}
async fn update_member(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMember>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Member(id, input)).await
}
async fn create_edge(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<CreateGraphEdge>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Edge(input)).await
}

async fn export_data(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Response> {
    let exported = company::data::export(&scoped(&state, &context)).await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"ai-center-company-{}.json\"",
                    context.workspace_id
                ),
            ),
            (axum::http::header::CACHE_CONTROL, "no-store".into()),
        ],
        Json(exported),
    )
        .into_response())
}
async fn archived_projects(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<crate::models::ProjectSummary>>> {
    Ok(Json(
        company::data::archived_projects(&scoped(&state, &context)).await?,
    ))
}
async fn archive_project(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<company::data::ArchiveProject>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Archive(id, input)).await
}
async fn export_project(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let exported = company::project_data::export(&scoped(&state, &context), id).await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"ai-center-project-{id}.json\""),
            ),
            (axum::http::header::CACHE_CONTROL, "no-store".into()),
        ],
        Json(exported),
    )
        .into_response())
}
async fn rename_project(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<company::data::RenameProject>,
) -> AppResult<Response> {
    mutate(state, context, key, Command::Rename(id, input)).await
}
