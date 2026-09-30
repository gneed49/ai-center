import type { ProjectSummary, WorkspaceSummary } from "./types";

export interface CompanyInput {
  name: string;
  description: string;
}
export interface CompanyMember {
  public_id: string;
  actor_id: string;
  role: "owner" | "editor" | "viewer";
  invitation_status: "pending" | "accepted" | "revoked";
  accepted_at: string | null;
}
export interface CompanyAgent {
  node_public_id: string;
  project_public_id: string;
  scope_kind: "company" | "project";
  node_key: string;
  profile_key: string;
  name: string;
  role: string;
  requires_context_pack: boolean;
}
export interface CompanyOverview {
  workspace: WorkspaceSummary & { description: string };
  company_scope: { kind: "company"; project_public_id: string } | null;
  agents: CompanyAgent[];
  projects: ProjectSummary[];
  members: CompanyMember[];
  setup_complete: boolean;
}
export type GraphKind =
  | "session"
  | "task"
  | "scope"
  | "agent"
  | "knowledge"
  | "artifact"
  | "external_reference"
  | "insight";
export interface GraphNode {
  id: string;
  kind: GraphKind;
  project_public_id: string;
  scope_kind: "company" | "project";
  label: string;
  status: string;
  version_public_id: string | null;
  version_number?: number | null;
  source_url: string | null;
  app_path?: string | null;
}
export interface GraphEdge {
  id: string;
  source_kind: GraphKind;
  source_public_id: string;
  source_project_public_id: string;
  target_kind: GraphKind;
  target_public_id: string;
  target_project_public_id: string;
  edge_type: string;
  status: string;
  provenance: Record<string, unknown>;
}
export interface GraphView {
  workspace_public_id: string;
  project_public_id: string | null;
  nodes: GraphNode[];
  edges: GraphEdge[];
  source_graph_versions: Array<{
    project_public_id: string;
    graph_version: number;
  }>;
  truncated: boolean;
  limits: { max_nodes: number; max_edges: number };
}
export interface GraphEndpoint {
  kind: GraphKind;
  public_id: string;
  project_public_id: string;
}
export interface CreateGraphEdge {
  public_id: string;
  source: GraphEndpoint;
  target: GraphEndpoint;
  edge_type: string;
}

export interface GraphSourceView {
  public_id: string;
  project_public_id: string;
  project_name: string;
  scope_kind: "company" | "project";
  kind: "knowledge" | "context_pack" | "task" | "artifact";
  title: string;
  status: string;
  version_number: number | null;
  recorded_at: string;
  content: Record<string, unknown>;
  links: { label: string; app_path: string }[];
}

export interface KnowledgeLibraryItem {
  public_id: string;
  project_public_id: string;
  project_name: string;
  project_status: string;
  scope_kind: "company" | "project";
  title: string;
  excerpt: string;
  entry_type: string;
  status: string;
  version_number: number;
  recorded_at: string;
}
export interface KnowledgeLibrary {
  items: KnowledgeLibraryItem[];
  total: number;
  limit: number;
  offset: number;
}
