export const artifactTypes = [
  "kickoff",
  "specification",
  "product_tickets",
  "technical_plan",
  "technical_tickets",
] as const;
export type ArtifactType = (typeof artifactTypes)[number];
export type ArtifactStatus = "draft" | "validated";
export type ArtifactSourceKind =
  "knowledge" | "context_pack" | "deliverable" | "session" | "artifact_version";
export interface ArtifactSourceInput {
  kind: ArtifactSourceKind;
  public_id: string;
}
export interface ArtifactSource extends ArtifactSourceInput {
  project_id: string;
  artifact_id?: string;
  title: string;
  version?: number | null;
  status_at_capture?: string;
  content_hash?: string;
  scope?: string;
  origin_only?: boolean;
  captured_at?: string;
  message_count?: number;
  last_message_id?: string | null;
  last_message_at?: string | null;
}
export interface ArtifactContent {
  title: string;
  body_markdown: string;
  structured_content: Record<string, unknown>;
  sources: ArtifactSourceInput[];
}
export interface CreateArtifact extends ArtifactContent {
  artifact_type: ArtifactType;
}
export interface SaveArtifact extends ArtifactContent {
  expected_version_id: string;
}
export interface ArtifactSummary {
  public_id: string;
  project_id: string;
  artifact_type: ArtifactType;
  title: string;
  status: ArtifactStatus;
  current_version_id: string;
  version: number;
  created_at: string;
  updated_at: string;
}
export interface ArtifactVersion extends Omit<ArtifactContent, "sources"> {
  public_id: string;
  version: number;
  sources: ArtifactSource[];
  status: ArtifactStatus;
  created_by_actor_id: string;
  created_at: string;
  validated_at: string | null;
  content_hash: string;
}
export interface ArtifactDetail {
  artifact: ArtifactSummary;
  current_version: ArtifactVersion;
}
export interface ArtifactPage<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}
export interface ArtifactFilters {
  q?: string;
  type?: ArtifactType;
  status?: ArtifactStatus;
  limit?: number;
  offset?: number;
}
export type DestinationProvider = "internal" | "notion" | "linear" | "github";
export interface ArtifactDestination {
  artifact_type: ArtifactType;
  provider: DestinationProvider;
  target_id: string | null;
  label: string;
  origin: "default" | "company" | "project";
  revision: number;
  project_id: string | null;
}
export interface SetArtifactDestination {
  artifact_type: ArtifactType;
  provider: DestinationProvider;
  target_id: string | null;
  label: string;
  expected_revision: number;
}
export interface ArtifactDestinations {
  items: ArtifactDestination[];
}
