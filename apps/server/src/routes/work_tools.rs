//! Private work-tool credentials and explicit artifact publication commands.
use super::{
    ApiState, AppResult, Extension, Json, MutationStart, Path, Query, RequestContext, Response,
    Router, State, Uuid, begin_idempotent, finalize_mutation, get, mutation_start, post, scoped,
};
use crate::work_tools::{
    self, DisableConnection, ListPublications, PublishArtifact, ReconcilePublication,
    SaveConnection,
};
use serde_json::{Value, json};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/work-tools", get(settings))
        .route("/api/work-tools/connections", post(save))
        .route("/api/work-tools/connections/{id}/disable", post(disable))
        .route("/api/work-tools/connections/{id}/test", post(test))
        .route("/api/artifacts/{id}/publications", get(list).post(publish))
        .route(
            "/api/artifacts/{id}/ticket-publications",
            get(ticket_coverage).post(publish_tickets),
        )
        .route(
            "/api/artifacts/{id}/ticket-publications/preview",
            post(ticket_preview),
        )
        .route(
            "/api/artifacts/{id}/ticket-publication-commands/{key}",
            get(ticket_receipt),
        )
        .route("/api/publications/{id}", get(detail))
        .route("/api/publications/{id}/reconcile", post(reconcile))
        .route("/api/publications/{id}/refresh", post(refresh))
        .route("/api/publications/{id}/cancel", post(cancel))
        .route(
            "/api/projects/{id}/code-observations",
            get(code_list).post(code_read),
        )
        .route("/api/code-observations/{id}", get(code_detail))
}
async fn settings(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<work_tools::Settings>> {
    Ok(Json(work_tools::settings(&scoped(&state, &context)).await?))
}
async fn list(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListPublications>,
) -> AppResult<Json<work_tools::Publications>> {
    Ok(Json(
        work_tools::list(&scoped(&state, &context), id, query).await?,
    ))
}
async fn detail(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<work_tools::PublicationDetail>> {
    Ok(Json(
        work_tools::detail(&scoped(&state, &context), id).await?,
    ))
}
enum Command {
    Save(SaveConnection),
    Disable(Uuid, DisableConnection),
    Test(Uuid),
    Publish(Uuid, PublishArtifact),
    PublishTickets(Uuid, work_tools::ticket_models::PublishTickets),
    Reconcile(Uuid, ReconcilePublication),
    Refresh(Uuid),
    Cancel(Uuid),
    ReadCode(Uuid, work_tools::code::ReadCode),
}
impl Command {
    fn request(&self, state: &crate::service::AppState) -> AppResult<Value> {
        Ok(match self {
            Self::Save(input) => {
                crate::providers::request_commitment(state, input, input.api_key.as_ref())?
            }
            Self::Disable(id, input) => json!({"id":id,"input":input}),
            Self::Publish(id, input) => json!({"artifact_id":id,"input":input}),
            Self::PublishTickets(id, input) => json!({"artifact_id":id,"input":input}),
            Self::Reconcile(id, input) => json!({"publication_id":id,"input":input}),
            Self::Test(id) | Self::Refresh(id) | Self::Cancel(id) => json!({"id":id}),
            Self::ReadCode(id, input) => json!({"project_id":id,"input":input}),
        })
    }
    fn operation(&self) -> String {
        if let Self::PublishTickets(id, _) = self {
            return work_tools::tickets::operation(*id);
        }
        let family = match self {
            Self::Save(_) => "work_tool.connection.save",
            Self::Disable(..) => "work_tool.connection.disable",
            Self::Test(_) => "work_tool.connection.test",
            Self::Publish(..) => "publication.create",
            Self::Reconcile(..) => "publication.reconcile",
            Self::Refresh(_) => "publication.refresh",
            Self::Cancel(_) => "publication.cancel",
            Self::ReadCode(..) => "github_code.read",
            Self::PublishTickets(..) => unreachable!("qualified operation handled above"),
        };
        family.into()
    }
}
async fn command(
    state: ApiState,
    context: RequestContext,
    key: Uuid,
    command: Command,
) -> AppResult<Response> {
    let scope = scoped(&state, &context);
    let project = if let Command::PublishTickets(id, _) = &command {
        Some(work_tools::tickets::project(&scope, *id).await?)
    } else {
        None
    };
    let lease = match mutation_start(
        begin_idempotent(
            &state,
            &context,
            project,
            &command.operation(),
            key,
            &command.request(&scope)?,
        )
        .await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let result: AppResult<Value> = crate::idempotency::with_lease(&scope, &lease, async {
        match command {
            Command::Save(input) => work_tools::save_connection(&scope, input, Some(&lease))
                .await
                .map(|value| json!(value)),
            Command::Disable(id, input) => {
                work_tools::disable_connection(&scope, id, input, Some(&lease))
                    .await
                    .map(|value| json!(value))
            }
            Command::Test(id) => work_tools::test_connection(&scope, id)
                .await
                .map(|value| json!(value)),
            Command::PublishTickets(id, input) => {
                work_tools::tickets::publish(&scope, id, input, Some(&lease))
                    .await
                    .map(|value| json!(value))
            }
            Command::Publish(id, input) => work_tools::publish(&scope, id, input, Some(&lease))
                .await
                .map(|value| json!(value)),
            Command::Reconcile(id, input) => work_tools::reconcile(&scope, id, input, Some(&lease))
                .await
                .map(|value| json!(value)),
            Command::Refresh(id) => work_tools::refresh(&scope, id, Some(&lease))
                .await
                .map(|value| json!(value)),
            Command::Cancel(id) => work_tools::cancel(&scope, id, Some(&lease))
                .await
                .map(|value| json!(value)),
            Command::ReadCode(id, input) => work_tools::code::read(&scope, id, input, Some(&lease))
                .await
                .map(|value| json!(value)),
        }
    })
    .await;
    finalize_mutation(&state, &context, &lease, result).await
}
async fn save(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Json(input): Json<SaveConnection>,
) -> AppResult<Response> {
    command(s, c, k, Command::Save(input)).await
}
async fn disable(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<DisableConnection>,
) -> AppResult<Response> {
    command(s, c, k, Command::Disable(id, input)).await
}
async fn test(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(s, c, k, Command::Test(id)).await
}
async fn publish(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<PublishArtifact>,
) -> AppResult<Response> {
    command(s, c, k, Command::Publish(id, input)).await
}
async fn reconcile(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<ReconcilePublication>,
) -> AppResult<Response> {
    command(s, c, k, Command::Reconcile(id, input)).await
}
async fn refresh(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(s, c, k, Command::Refresh(id)).await
}
async fn cancel(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(s, c, k, Command::Cancel(id)).await
}
async fn code_list(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<work_tools::code::CodeQuery>,
) -> AppResult<Json<work_tools::code::CodeList>> {
    Ok(Json(
        work_tools::code::list(&scoped(&s, &c), id, query).await?,
    ))
}
async fn code_detail(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<work_tools::code::CodeDetail>> {
    Ok(Json(work_tools::code::detail(&scoped(&s, &c), id).await?))
}
async fn code_read(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<work_tools::code::ReadCode>,
) -> AppResult<Response> {
    command(s, c, k, Command::ReadCode(id, input)).await
}

/// Only this authenticated read-only POST may omit the command key.
pub(super) fn is_ticket_preview(path: &str) -> bool {
    path.strip_prefix("/api/artifacts/")
        .and_then(|tail| tail.strip_suffix("/ticket-publications/preview"))
        .is_some_and(|id| Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id))
}
async fn ticket_coverage(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<work_tools::ticket_models::TicketCoverageQuery>,
) -> AppResult<Json<work_tools::ticket_models::TicketCoverage>> {
    Ok(Json(
        work_tools::tickets::coverage(&scoped(&s, &c), id, query).await?,
    ))
}
async fn ticket_preview(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Json(input): Json<work_tools::ticket_models::PreviewTickets>,
) -> AppResult<Json<work_tools::ticket_models::TicketPreview>> {
    Ok(Json(
        work_tools::tickets::preview(&scoped(&s, &c), id, input).await?,
    ))
}
async fn publish_tickets(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(k): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(mut input): Json<work_tools::ticket_models::PublishTickets>,
) -> AppResult<Response> {
    work_tools::tickets::canonicalize_command(&mut input)?;
    command(s, c, k, Command::PublishTickets(id, input)).await
}
async fn ticket_receipt(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path((id, key)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        work_tools::tickets::receipt(&scoped(&s, &c), id, key).await?,
    ))
}
