//! Private provider settings use the same authentication and mutation leases.
use super::{
    ApiState, AppError, AppResult, Extension, IdempotencyLease, IntoResponse, Json, MutationStart,
    Path, RequestContext, Response, Router, State, StatusCode, Uuid, Value, begin_idempotent,
    finish_idempotent, get, idempotency, json, mutation_start, post, scoped, service,
};
use crate::providers::{self, CreateConnection, Selection, UpdateConnection};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/ai/settings", get(settings))
        .route("/api/ai/connections", post(create))
        .route("/api/ai/connections/{id}/update", post(update))
        .route("/api/ai/connections/{id}/delete", post(delete))
        .route("/api/ai/connections/{id}/test", post(test))
        .route("/api/ai/selection", post(select))
        .route(
            "/api/ai/connections/{id}/subscription/status",
            get(subscription_status),
        )
        .route(
            "/api/ai/connections/{id}/subscription/login",
            post(subscription_login),
        )
        .route(
            "/api/ai/connections/{id}/subscription/login/{login_id}",
            get(subscription_login_status),
        )
        .route(
            "/api/ai/connections/{id}/subscription/login/{login_id}/cancel",
            post(subscription_cancel),
        )
        .route(
            "/api/ai/connections/{id}/subscription/logout",
            post(subscription_logout),
        )
}
async fn settings(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<providers::Settings>> {
    Ok(Json(providers::settings(&scoped(&state, &context)).await?))
}

enum Command {
    Create(CreateConnection),
    Update(Uuid, UpdateConnection),
    Delete(Uuid),
    Select(Selection),
    Test(Uuid),
    Login(Uuid),
    Cancel(Uuid, Uuid),
    Logout(Uuid),
}
impl Command {
    fn request(&self, state: &service::AppState) -> AppResult<Value> {
        match self {
            Self::Create(input) => {
                providers::request_commitment(state, input, input.api_key.as_ref())
            }
            Self::Update(id, input) => Ok(
                json!({"id":id,"update":providers::request_commitment(state,input,input.api_key.as_ref())?}),
            ),
            Self::Select(input) => Ok(json!(input)),
            Self::Cancel(id, login_id) => Ok(json!({"id":id,"login_id":login_id})),
            Self::Delete(id) | Self::Test(id) | Self::Login(id) | Self::Logout(id) => {
                Ok(json!({"id":id}))
            }
        }
    }
    const fn operation(&self) -> &'static str {
        match self {
            Self::Create(_) => "provider_connection.create",
            Self::Update(..) => "provider_connection.update",
            Self::Delete(_) => "provider_connection.delete",
            Self::Select(_) => "provider_selection.update",
            Self::Test(_) => "provider_connection.test",
            Self::Login(_) => "provider_subscription.login",
            Self::Cancel(..) => "provider_subscription.cancel",
            Self::Logout(_) => "provider_subscription.logout",
        }
    }
    async fn run(self, state: &service::AppState, lease: &IdempotencyLease) -> AppResult<Value> {
        match self {
            Self::Create(input) => Ok(json!(providers::create(state, input, Some(lease)).await?)),
            Self::Update(id, input) => Ok(json!(
                providers::update(state, id, input, Some(lease)).await?
            )),
            Self::Delete(id) => providers::delete(state, id, Some(lease)).await,
            Self::Select(input) => Ok(json!(providers::select(state, input, Some(lease)).await?)),
            Self::Test(id) => Ok(json!(
                idempotency::with_optional_lease(state, Some(lease), providers::test(state, id))
                    .await?
            )),
            Self::Login(id) => {
                let (runtime, scope) = providers::subscription(state, id).await?;
                Ok(json!(runtime.start_login(scope).await?))
            }
            Self::Cancel(id, login_id) => {
                let (runtime, scope) = providers::subscription(state, id).await?;
                Ok(json!(runtime.cancel_login(scope, login_id).await?))
            }
            Self::Logout(id) => {
                let (runtime, scope) = providers::subscription(state, id).await?;
                Ok(json!(runtime.logout(scope).await?))
            }
        }
    }
}
async fn command(
    state: ApiState,
    context: RequestContext,
    key: Uuid,
    command: Command,
) -> AppResult<Response> {
    let service = scoped(&state, &context);
    let request = command.request(&service)?;
    let lease = match mutation_start(
        begin_idempotent(&state, &context, None, command.operation(), key, &request).await?,
    )? {
        MutationStart::Execute(lease) => lease,
        MutationStart::Respond(response) => return Ok(response),
    };
    let result = command.run(&service, &lease).await;
    // Login links only live in the isolated in-memory login session. Never
    // persist them in the idempotency response; GET login status can resume it.
    let stored = if let Ok(body) = &result {
        let mut body = body.clone();
        if let Some(object) = body.as_object_mut() {
            object.remove("auth_url");
        }
        Ok(body)
    } else {
        let stored = finish_idempotent(&state, &context, &lease, &result).await?;
        return Ok((
            StatusCode::from_u16(stored.status_code)
                .map_err(|_| AppError::Internal("stored status is invalid".into()))?,
            Json(stored.body),
        )
            .into_response());
    };
    finish_idempotent(&state, &context, &lease, &stored).await?;
    Ok(Json(result?).into_response())
}
async fn create(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<CreateConnection>,
) -> AppResult<Response> {
    command(state, context, key, Command::Create(input)).await
}
async fn update(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateConnection>,
) -> AppResult<Response> {
    command(state, context, key, Command::Update(id, input)).await
}
async fn delete(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(state, context, key, Command::Delete(id)).await
}
async fn test(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(state, context, key, Command::Test(id)).await
}
async fn select(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<Selection>,
) -> AppResult<Response> {
    command(state, context, key, Command::Select(input)).await
}
async fn subscription_login(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(state, context, key, Command::Login(id)).await
}
async fn subscription_cancel(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path((id, login_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Response> {
    command(state, context, key, Command::Cancel(id, login_id)).await
}
async fn subscription_logout(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(state, context, key, Command::Logout(id)).await
}
async fn subscription_status(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let (runtime, scope) = providers::subscription(&scoped(&state, &context), id).await?;
    Ok(Json(json!(runtime.status(scope).await?)))
}
async fn subscription_login_status(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path((id, login_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let (runtime, scope) = providers::subscription(&scoped(&state, &context), id).await?;
    Ok(Json(json!(runtime.login_status(scope, login_id).await?)))
}
