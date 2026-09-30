use super::{
    ApiState, AppResult, Extension, Json, MutationStart, Path, RequestContext, Response, Router,
    State, Uuid, Value, begin_idempotent, finalize_mutation, get, mutation_start, patch, post,
    scoped,
};
use crate::team::{
    self, AcceptInvitation, CreateInvitation, Invitation, InvitationPreview, PreviewInvitation,
    SetProfile, TeamMember,
};
use serde_json::json;

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/api/team/members", get(members))
        .route("/api/team/profile", patch(profile))
        .route("/api/team/invitations", get(invitations).post(create))
        .route("/api/team/invitations/{id}/revoke", post(revoke))
        .route("/api/team/invitations/{id}/preview", post(preview))
        .route("/api/team/invitations/{id}/accept", post(accept))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LinkOperation {
    Preview,
    Accept,
}

pub(super) fn link_operation(path: &str) -> Option<LinkOperation> {
    let rest = path.strip_prefix("/api/team/invitations/")?;
    let (id, action) = rest.split_once('/')?;
    Uuid::parse_str(id).ok()?;
    match action {
        "preview" => Some(LinkOperation::Preview),
        "accept" => Some(LinkOperation::Accept),
        _ => None,
    }
}

async fn members(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<TeamMember>>> {
    Ok(Json(team::members(&scoped(&state, &context)).await?))
}
async fn profile(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<SetProfile>,
) -> AppResult<Json<TeamMember>> {
    Ok(Json(team::profile(&scoped(&state, &context), input).await?))
}
async fn invitations(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<Invitation>>> {
    Ok(Json(team::invitations(&scoped(&state, &context)).await?))
}
async fn preview(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(input): Json<PreviewInvitation>,
) -> AppResult<Json<InvitationPreview>> {
    Ok(Json(team::preview(&state.service, id, input).await?))
}
async fn accept(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<Uuid>,
    Json(input): Json<AcceptInvitation>,
) -> AppResult<Json<crate::models::WorkspaceSummary>> {
    Ok(Json(
        team::accept(&scoped(&state, &context), id, input).await?,
    ))
}

enum Command {
    Create(CreateInvitation),
    Revoke(Uuid),
}
impl Command {
    const fn operation(&self) -> &'static str {
        match self {
            Self::Create(_) => "team.invitation.create",
            Self::Revoke(_) => "team.invitation.revoke",
        }
    }
    fn request(&self) -> Value {
        match self {
            Self::Create(input) => json!(input),
            Self::Revoke(id) => json!({"invitation_id":id}),
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
    let result = match command {
        Command::Create(input) => team::create(&scoped, input, Some(&lease))
            .await
            .map(|value| json!(value)),
        Command::Revoke(id) => team::revoke(&scoped, id, Some(&lease))
            .await
            .map(|value| json!(value)),
    };
    finalize_mutation(&state, &context, &lease, result).await
}
async fn create(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Json(input): Json<CreateInvitation>,
) -> AppResult<Response> {
    command(state, context, key, Command::Create(input)).await
}
async fn revoke(
    State(state): State<ApiState>,
    Extension(context): Extension<RequestContext>,
    Extension(key): Extension<Uuid>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    command(state, context, key, Command::Revoke(id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_exceptions_match_only_the_two_valid_link_endpoints() {
        let root = format!("/api/team/invitations/{}", Uuid::new_v4());
        assert!(link_operation(&format!("{root}/preview")).is_some());
        assert!(link_operation(&format!("{root}/accept")).is_some());
        for path in [
            format!("{root}/accept/extra"),
            format!("{root}/revoke"),
            "/api/team/invitations/bad/accept".into(),
            "/api/projects".into(),
        ] {
            assert!(link_operation(&path).is_none());
        }
    }
}
