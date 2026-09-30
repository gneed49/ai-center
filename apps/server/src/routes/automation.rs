use super::{
    ApiState, AppResult, Extension, Json, MutationStart, RequestContext, Response, Router, State,
    Uuid, begin_idempotent, finalize_mutation, get, mutation_start, scoped,
};
use crate::automation::{self, SetControl, Status};
use serde_json::json;
pub(super) fn router() -> Router<ApiState> {
    Router::new().route("/api/automation", get(status).post(set))
}
async fn status(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Status>> {
    Ok(Json(automation::status(&scoped(&state, &context)).await?))
}
async fn set(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<SetControl>,
) -> AppResult<Response> {
    let lease = match mutation_start(
        begin_idempotent(&state, &context, None, "automation.control", key, &input).await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let result = automation::set(&scoped(&state, &context), input, Some(&lease))
        .await
        .map(|receipt| json!(receipt));
    finalize_mutation(&state, &context, &lease, result).await
}
