// [FICTIF] Local contract fixture; no real account or credential.
import type {
  SourceObservationDetail,
  ToolSourceReference,
  SourceCommandResult,
} from "@/api/tool-source-types";
export const fixtureSourceObservation: SourceObservationDetail = {
  observation: {
    source_kind: "tool_source_observation",
    public_id: "40000000-0000-4000-8000-000000000001",
    reference_public_id: "30000000-0000-4000-8000-000000000001",
    publication_public_id: null,
    source_project_public_id: "20000000-0000-4000-8000-000000000001",
    provider: "linear",
    object_kind: "issue",
    external_id: "70000000-0000-4000-8000-000000000001",
    canonical_url:
      "https://linear.app/fictif/issue/PROD-42/specification-fictive",
    version: 1,
    observed_at: "2026-09-24T00:00:00Z",
    remote_updated_at: "2026-09-23T23:50:00Z",
    title: "[FICTIF] Durée des crédits",
    excerpt:
      "[FICTIF] Les crédits acquis restent disponibles sans date d’expiration.\n\nLe ticket décrit la règle attendue ; il ne prouve pas son implémentation.",
    availability: "available",
    content_hash:
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    snapshot_hash:
      "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    projection_version: "existing-tool-text-v1",
    trust: "observed_external",
    mandatory: false,
    coverage: "complete",
    omission_reasons: [
      "attachments_not_read",
      "comments_not_read",
      "related_objects_not_read",
    ],
    freshness: {
      is_current: true,
      eligible: true,
      reasons: [],
      current_observation_id: "40000000-0000-4000-8000-000000000001",
      last_checked_at: "2026-09-24T00:10:00Z",
      last_check_status: "available",
      last_check_error_code: null,
    },
  },
  body_markdown:
    "[FICTIF] Les crédits acquis restent disponibles sans date d’expiration.\n\nLe ticket décrit la règle attendue ; il ne prouve pas son implémentation.",
  metadata: {
    identifier: "PROD-42",
    team_id: "90000000-0000-4000-8000-000000000001",
    state: {
      id: "90000000-0000-4000-8000-000000000002",
      name: "[FICTIF] À faire",
      type: "unstarted",
    },
  },
};
export const fixtureSourceReference: ToolSourceReference = {
  public_id: "30000000-0000-4000-8000-000000000001",
  project_id: "20000000-0000-4000-8000-000000000001",
  provider: "linear",
  object_kind: "issue",
  external_id: "70000000-0000-4000-8000-000000000001",
  canonical_url:
    "https://linear.app/fictif/issue/PROD-42/specification-fictive",
  connection_id: "60000000-0000-4000-8000-000000000001",
  connection_revision: 2,
  status: "active",
  revision: 2,
  current_observation_id: "40000000-0000-4000-8000-000000000001",
  created_at: "2026-09-24T00:00:00Z",
  updated_at: "2026-09-24T00:10:00Z",
  last_attempt_at: "2026-09-24T00:09:50Z",
  last_checked_at: "2026-09-24T00:10:00Z",
  last_check_status: "available",
  last_check_error_code: null,
};
export const fixtureSourceResult: SourceCommandResult = {
  action: "attach",
  effect: "created",
  verification_status: "available",
  reference: fixtureSourceReference,
  observation: fixtureSourceObservation.observation,
};
