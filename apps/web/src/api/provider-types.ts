export interface Provider {
  id: string;
  name: string;
  auth_method: "api_key" | "subscription";
  available: boolean;
  unavailable_reason?: string | null;
  help_url?: string | null;
}

export interface ProviderConnection {
  id: string;
  provider: string;
  name: string;
  model: string;
  key_hint: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProviderSelection {
  mode: "server_default" | "deterministic" | "connection";
  connection_id: string | null;
}

export interface ProviderSettings {
  providers: Provider[];
  connections: ProviderConnection[];
  selection: ProviderSelection;
  storage: { available: boolean; message?: string | null };
}

export interface ConnectionInput {
  id: string;
  provider: string;
  name: string;
  model: string;
  api_key?: string;
}

export interface ProviderModel {
  id: string;
  name: string;
}

export interface SubscriptionStatus {
  status: "disconnected" | "connected" | "unavailable" | "unknown";
  connected: boolean;
  available: boolean;
  message?: string;
}

export interface SubscriptionLogin {
  login_id: string;
  status: "pending" | "connected" | "failed" | "cancelled" | "expired";
  auth_url?: string | null;
}
