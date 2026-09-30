export type SourceProvider = "linear" | "notion";
export type ObservationKind =
  "tool_source_observation" | "publication_observation";
export type SourceCoverage = "complete" | "partial" | "none";
export type VerificationStatus =
  "available" | "partial" | "unavailable" | "failed";
export interface SourceFreshness {
  is_current: boolean;
  eligible: boolean;
  reasons: string[];
  current_observation_id: string | null;
  last_checked_at: string | null;
  last_check_status: VerificationStatus | null;
  last_check_error_code: string | null;
}
export interface SourceObservation {
  source_kind: ObservationKind;
  public_id: string;
  reference_public_id: string | null;
  publication_public_id: string | null;
  source_project_public_id: string;
  provider: SourceProvider;
  object_kind: "issue" | "page";
  external_id: string | null;
  canonical_url: string;
  version: number;
  observed_at: string;
  remote_updated_at: string | null;
  title: string;
  excerpt: string;
  availability: "available" | "unavailable";
  content_hash: string;
  snapshot_hash: string;
  projection_version: string;
  trust: "observed_external";
  mandatory: false;
  coverage: SourceCoverage;
  omission_reasons: string[];
  freshness: SourceFreshness;
}
export interface SourceObservationDetail {
  observation: SourceObservation;
  body_markdown: string;
  metadata: Record<string, unknown>;
}
export interface ToolSourceReference {
  public_id: string;
  project_id: string;
  provider: SourceProvider;
  object_kind: "issue" | "page";
  external_id: string;
  canonical_url: string;
  connection_id: string;
  connection_revision: number;
  status: "active" | "detached";
  revision: number;
  current_observation_id: string;
  created_at: string;
  updated_at: string;
  last_attempt_at: string | null;
  last_checked_at: string | null;
  last_check_status: VerificationStatus;
  last_check_error_code: string | null;
}
export interface ToolSourceDetail {
  reference: ToolSourceReference;
  observation: SourceObservation;
}
export interface SourcePage<T> {
  items: T[];
  next_cursor: string | null;
  total_count: number;
  limit: number;
}
export interface ToolSourceList extends SourcePage<ToolSourceDetail> {
  active_count: number;
}
export interface AttachToolSource {
  connection_id: string;
  expected_connection_revision: number;
  provider: SourceProvider;
  source: string;
  confirm_scope_sharing: true;
}
export interface ExpectedSource {
  expected_revision: number;
  expected_observation_id: string;
}
export interface RebindToolSource extends ExpectedSource {
  connection_id: string;
  expected_connection_revision: number;
  confirm_scope_sharing: true;
}
export type SourceAction = "attach" | "refresh" | "detach" | "rebind";
export interface SourceCommandResult extends ToolSourceDetail {
  action: SourceAction;
  effect:
    "created" | "existing" | "changed" | "unchanged" | "detached" | "rebound";
  verification_status: VerificationStatus | "not_performed";
}
export interface SourceCommandReceipt {
  status:
    | "not_received"
    | "processing"
    | "interrupted"
    | "retryable"
    | "completed"
    | "failed"
    | "expired";
  can_retry: boolean;
  retry_after: string | null;
  result: SourceCommandResult | null;
  error: { code: string; message: string } | null;
}
