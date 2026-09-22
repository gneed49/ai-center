import { request } from "./client";
import type {
  CompanyInput,
  CompanyMember,
  CompanyOverview,
  CreateGraphEdge,
  GraphEdge,
  GraphView,
  GraphSourceView,
} from "./company-types";
import type { WorkspaceSummary } from "./types";

function command(method: string, input: unknown, key: string): RequestInit {
  return {
    method,
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}

export const companyApi = {
  capabilities: () =>
    request<{ can_create_company: boolean }>("/api/workspaces/capabilities"),
  overview: () => request<CompanyOverview>("/api/company"),
  setup: (input: CompanyInput, key: string) =>
    request<CompanyOverview>("/api/company/setup", command("POST", input, key)),
  update: (input: CompanyInput, key: string) =>
    request<CompanyOverview>("/api/company", command("PATCH", input, key)),
  create: (input: CompanyInput & { public_id: string }, key: string) =>
    request<WorkspaceSummary>("/api/workspaces", command("POST", input, key)),
  members: () => request<CompanyMember[]>("/api/company/members"),
  updateMember: (
    id: string,
    input: {
      role?: CompanyMember["role"];
      invitation_status?: CompanyMember["invitation_status"];
    },
    key: string,
  ) =>
    request<CompanyMember>(
      `/api/company/members/${encodeURIComponent(id)}`,
      command("PATCH", input, key),
    ),
  source: (project: string, kind: string, id: string) =>
    request<GraphSourceView>(
      `/api/projects/${encodeURIComponent(project)}/sources/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`,
    ),
  graph: (projectId?: string) =>
    request<GraphView>(
      projectId
        ? `/api/projects/${encodeURIComponent(projectId)}/graph`
        : "/api/company/graph",
    ),
  createEdge: (input: CreateGraphEdge, key: string) =>
    request<GraphEdge>("/api/graph/edges", command("POST", input, key)),
};
