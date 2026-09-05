type RequestIdentity = {
  actorId: string | null;
  workspaceId: string | null;
  accessToken: string | null;
  controller: AbortController;
};

let identity: RequestIdentity | null = null;

export class RequestContextChangedError extends Error {
  constructor() {
    super("Le contexte de connexion a changé.");
    this.name = "AbortError";
  }
}

/** Replace the request boundary before publishing the new React identity. */
export function setRequestIdentity(
  actorId: string | null,
  workspaceId: string | null,
  accessToken: string | null,
) {
  if (identity?.actorId === actorId && identity.workspaceId === workspaceId) {
    // Refreshing a token does not discard drafts or another valid response.
    identity.accessToken = accessToken;
    return false;
  }
  identity?.controller.abort();
  identity = {
    actorId,
    workspaceId,
    accessToken,
    controller: new AbortController(),
  };
  return true;
}

export function captureRequestContext() {
  const captured = identity;
  const token = captured
    ? captured.accessToken
    : window.localStorage.getItem("ai-center.access-token");
  const workspaceId = captured
    ? captured.workspaceId
    : (window.localStorage.getItem("ai-center.workspace-id") ??
      import.meta.env.VITE_WORKSPACE_ID);
  return {
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(workspaceId ? { "X-AI-Center-Workspace-Id": workspaceId } : {}),
    } as Record<string, string>,
    signal: captured?.controller.signal,
    assertCurrent() {
      if (captured !== identity || captured?.controller.signal.aborted) {
        throw new RequestContextChangedError();
      }
    },
  };
}
