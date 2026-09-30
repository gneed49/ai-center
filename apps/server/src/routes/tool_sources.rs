//! Local source readers and explicitly confirmed durable remote commands.
use super::{
    ApiState, AppResult, Extension, Json, MutationStart, Path, Query, RequestContext, Response,
    Router, State, Uuid, begin_idempotent, finalize_mutation, get, mutation_start, post, scoped,
};
use crate::work_tools::sources::{
    commands::{self, Command},
    models::{
        AttachToolSource, ExpectedSource, ListObservations, ListSources, RebindToolSource,
        SourceKind, SourceObservationDetail, SourceObservations, ToolSourceDetail, ToolSources,
    },
    read,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/projects/{id}/tool-sources", get(list).post(attach))
        .route("/api/tool-sources/{id}", get(detail))
        .route("/api/tool-sources/{id}/observations", get(history))
        .route("/api/tool-source-observations/{id}", get(observation))
        .route(
            "/api/publication-observations/{id}",
            get(publication_observation),
        )
        .route("/api/tool-sources/{id}/refresh", post(refresh))
        .route("/api/tool-sources/{id}/detach", post(detach))
        .route("/api/tool-sources/{id}/rebind", post(rebind))
        .route(
            "/api/projects/{id}/tool-source-commands/{key}",
            get(receipt),
        )
}
async fn list(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<ListSources>,
) -> AppResult<Json<ToolSources>> {
    Ok(Json(read::list(&scoped(&s, &c), id, q).await?))
}
async fn detail(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ToolSourceDetail>> {
    Ok(Json(read::detail(&scoped(&s, &c), id).await?))
}
async fn history(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<ListObservations>,
) -> AppResult<Json<SourceObservations>> {
    Ok(Json(read::history(&scoped(&s, &c), id, q).await?))
}
async fn observation(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<SourceObservationDetail>> {
    Ok(Json(
        read::observation_detail(&scoped(&s, &c), SourceKind::ToolSourceObservation, id).await?,
    ))
}
async fn publication_observation(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<SourceObservationDetail>> {
    Ok(Json(
        read::observation_detail(&scoped(&s, &c), SourceKind::PublicationObservation, id).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptQuery {
    operation: String,
}
async fn receipt(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Path((id, key)): Path<(Uuid, Uuid)>,
    Query(q): Query<ReceiptQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        commands::receipt(&scoped(&s, &c), id, key, &q.operation).await?,
    ))
}
async fn command(
    state: ApiState,
    context: RequestContext,
    key: Uuid,
    project: Uuid,
    command: Command,
) -> AppResult<Response> {
    let scope = scoped(&state, &context);
    let command = command.normalize()?;
    crate::artifacts::editor(&scope)?;
    if command.action() == "rebind" && scope.workspace_role != "owner" {
        return Err(crate::error::AppError::Forbidden);
    }
    let lease = match mutation_start(
        begin_idempotent(
            &state,
            &context,
            Some(project),
            &commands::operation(command.action(), project)?,
            key,
            &command.request(project),
        )
        .await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let result = crate::idempotency::with_lease(&scope, &lease, async {
        #[cfg(debug_assertions)]
        if let Some(reader) = &state.source_reader {
            return commands::execute_with_reader(&scope, project, command, &lease, reader)
                .await
                .map(|v| json!(v));
        }
        commands::execute(&scope, project, command, &lease)
            .await
            .map(|v| json!(v))
    })
    .await;
    finalize_mutation(&state, &context, &lease, result).await
}
async fn attach(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<AttachToolSource>,
) -> AppResult<Response> {
    command(s, c, key, id, Command::Attach(input)).await
}
async fn refresh(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<ExpectedSource>,
) -> AppResult<Response> {
    let project = commands::project_for_reference(&scoped(&s, &c), id).await?;
    command(s, c, key, project, Command::Refresh(id, input)).await
}
async fn detach(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<ExpectedSource>,
) -> AppResult<Response> {
    let project = commands::project_for_reference(&scoped(&s, &c), id).await?;
    command(s, c, key, project, Command::Detach(id, input)).await
}
async fn rebind(
    State(s): State<ApiState>,
    Extension(c): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<RebindToolSource>,
) -> AppResult<Response> {
    let project = commands::project_for_reference(&scoped(&s, &c), id).await?;
    command(s, c, key, project, Command::Rebind(id, input)).await
}
