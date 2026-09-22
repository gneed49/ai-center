import { request, requestText } from "./client";
import type { ProjectSummary } from "./types";
export interface AutomationSettings {
  enabled: boolean;
  generation: number;
  limits: {
    calls_per_hour: number;
    concurrent_calls: number;
    per_actor_calls_per_hour?: number;
    per_actor_concurrent_calls?: number;
    call_timeout_seconds: number;
  };
  calls_last_hour: number;
  active_calls: number;
  actor_calls_last_hour?: number;
  actor_active_calls?: number;
  known_estimated_cost_usd: number | null;
  runs_without_cost: number;
}
export interface AutomationCommand {
  enabled: boolean;
  expected_generation: number;
}
function command(input: unknown, key: string): RequestInit {
  return {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}
export const companyControlsApi = {
  automation: () => request<AutomationSettings>("/api/automation"),
  setAutomation: (input: AutomationCommand, key: string) =>
    request<Pick<AutomationSettings, "enabled" | "generation">>(
      "/api/automation",
      command(input, key),
    ),
  renameProject: (
    projectId: string,
    input: { name: string; expected_updated_at: string },
    key: string,
  ) =>
    request<ProjectSummary>(`/api/projects/${encodeURIComponent(projectId)}`, {
      ...command(input, key),
      method: "PATCH",
    }),
  archivedProjects: () =>
    request<ProjectSummary[]>("/api/company/projects/archived"),
  archiveProject: (projectId: string, archived: boolean, key: string) =>
    request<ProjectSummary>(
      `/api/projects/${encodeURIComponent(projectId)}/archive`,
      command({ archived }, key),
    ),
  exportProject: (projectId: string) =>
    requestText(`/api/projects/${encodeURIComponent(projectId)}/export`, {
      cache: "no-store",
    }),
  exportData: () => requestText("/api/company/export", { cache: "no-store" }),
};
