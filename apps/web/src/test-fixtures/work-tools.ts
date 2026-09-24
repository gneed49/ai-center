import type {
  Publication,
  WorkToolConnection,
  WorkToolSettings,
} from "@/api/work-tool-types";
export const fixtureToolConnection: WorkToolConnection = {
  public_id: "fixture-tool",
  provider: "notion",
  name: "[FICTIF] Documentation",
  enabled: true,
  revision: 2,
  created_at: "2026-09-21T10:00:00Z",
  updated_at: "2026-09-21T10:00:00Z",
};
export const fixtureTools: WorkToolSettings = {
  storage_available: true,
  connections: [fixtureToolConnection],
  capabilities: [
    {
      provider: "notion",
      create: true,
      read: true,
      reconcile: true,
      update: false,
    },
  ],
};
export const fixturePublication: Publication = {
  public_id: "fixture-publication",
  artifact_id: "fixture-artifact",
  version_id: "fixture-artifact-version-2",
  connection_id: "fixture-tool",
  provider: "notion",
  target_id: "fixture-notion-page",
  status: "queued",
  external_id: null,
  external_url: null,
  error_code: null,
  attempt_count: 0,
  created_at: "2026-09-21T11:00:00Z",
  updated_at: "2026-09-21T11:00:00Z",
};
