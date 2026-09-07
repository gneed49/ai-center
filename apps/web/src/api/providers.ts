import { createIdempotencyKey, request } from "./client";
import type {
  ConnectionInput,
  ProviderConnection,
  ProviderModel,
  ProviderSelection,
  ProviderSettings,
  SubscriptionLogin,
  SubscriptionStatus,
} from "./provider-types";

function privateRequest<T>(path: string, init?: RequestInit) {
  return request<T>(path, { ...init, cache: "no-store" });
}

function post<T>(path: string, body: unknown, key = createIdempotencyKey()) {
  return privateRequest<T>(path, {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(body),
  });
}

const connectionPath = (id: string) =>
  `/api/ai/connections/${encodeURIComponent(id)}`;

// Only masked metadata belongs in a query cache. Call secret mutations directly.
export const providersApi = {
  settings: (signal?: AbortSignal) =>
    privateRequest<ProviderSettings>("/api/ai/settings", { signal }),
  create: (input: ConnectionInput, key: string) =>
    post<ProviderConnection>("/api/ai/connections", input, key),
  update: (
    id: string,
    input: Omit<ConnectionInput, "id" | "provider">,
    key: string,
  ) => post<ProviderConnection>(`${connectionPath(id)}/update`, input, key),
  remove: (id: string) =>
    post<{ deleted: true }>(`${connectionPath(id)}/delete`, {}),
  test: (id: string) =>
    post<{ ok: true; models: ProviderModel[] }>(
      `${connectionPath(id)}/test`,
      {},
    ),
  select: (selection: ProviderSelection) =>
    post<ProviderSelection>("/api/ai/selection", selection),
  subscriptionStatus: (id: string, signal?: AbortSignal) =>
    privateRequest<SubscriptionStatus>(
      `${connectionPath(id)}/subscription/status`,
      {
        signal,
      },
    ),
  startLogin: (id: string) =>
    post<SubscriptionLogin>(`${connectionPath(id)}/subscription/login`, {}),
  loginStatus: (id: string, loginId: string, signal?: AbortSignal) =>
    privateRequest<SubscriptionLogin>(
      `${connectionPath(id)}/subscription/login/${encodeURIComponent(loginId)}`,
      { signal },
    ),
  cancelLogin: (id: string, loginId: string) =>
    post<SubscriptionLogin>(
      `${connectionPath(id)}/subscription/login/${encodeURIComponent(loginId)}/cancel`,
      {},
    ),
  logout: (id: string) =>
    post<SubscriptionStatus>(`${connectionPath(id)}/subscription/logout`, {}),
};
