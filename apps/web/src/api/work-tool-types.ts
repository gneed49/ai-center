export type WorkToolProvider = "notion" | "linear" | "github";
export interface WorkToolConnection {
  public_id: string;
  provider: WorkToolProvider;
  name: string;
  enabled: boolean;
  revision: number;
  created_at: string;
  updated_at: string;
}
export interface WorkToolSettings {
  storage_available: boolean;
  limits?: {
    max_pending_publications: number;
    max_publications_per_hour: number;
    max_remote_reads_per_hour: number;
    create_attempts_per_job: number;
    request_timeout_seconds: number;
  };
  usage?: {
    queued: number;
    processing: number;
    needs_review: number;
    publications_last_hour: number;
    remote_read_operations_last_hour: number;
    known_cost_usd: number | null;
  };
  connections: WorkToolConnection[];
  capabilities: {
    provider: WorkToolProvider;
    create: boolean;
    read: boolean;
    reconcile: boolean;
    update: boolean;
  }[];
}
export interface SaveWorkToolConnection {
  id: string;
  provider: WorkToolProvider;
  name: string;
  expected_revision: number;
  api_key?: string;
}
export interface WorkToolTest {
  ok: boolean;
  code: string;
}
export type PublicationStatus =
  | "queued"
  | "processing"
  | "succeeded"
  | "failed"
  | "needs_review"
  | "conflict"
  | "unavailable"
  | "cancelled";
export interface Publication {
  /** Older stored command receipts may omit these three fields. */
  source_ticket_index?: number;
  title?: string | null;
  source_version_number?: number | null;
  public_id: string;
  artifact_id: string;
  version_id: string;
  connection_id: string;
  provider: WorkToolProvider;
  target_id: string;
  status: PublicationStatus;
  external_id: string | null;
  external_url: string | null;
  error_code: string | null;
  attempt_count: number;
  created_at: string;
  updated_at: string;
}
export interface PublishArtifact {
  version_id: string;
  connection_id: string;
  expected_provider: WorkToolProvider;
  expected_target_id: string;
}
export interface Publications {
  items: Publication[];
  limit: number;
  offset: number;
}
export interface PublicationObservation {
  public_id: string;
  observed_at: string;
  observation_kind: string;
  external_id: string;
  external_url: string;
  remote_updated_at: string | null;
  snapshot: Record<string, unknown>;
}
export interface PublicationDetail {
  publication: Publication;
  observations: PublicationObservation[];
}
