import type {
  ArtifactDetail,
  ArtifactVersion,
  ArtifactDestinations,
} from "@/api/artifact-types";

/** Synthetic fixtures only. Never external documents or actual publication proof. */
export const fixtureArtifactVersion: ArtifactVersion = {
  public_id: "fixture-artifact-version-2",
  version: 2,
  title: "[FICTIF] Spécification atelier",
  body_markdown: "# [FICTIF] Version courante\nUne exigence confirmée.",
  structured_content: { requirement: "[FICTIF] Lecture seule" },
  sources: [
    {
      kind: "knowledge",
      public_id: "fixture-knowledge-version",
      project_id: "fixture-project",
      title: "[FICTIF] Exigence",
      version: 3,
      status_at_capture: "confirmed",
    },
  ],
  status: "draft",
  created_by_actor_id: "fixture-actor",
  created_at: "2026-09-21T11:00:00Z",
  validated_at: null,
  content_hash: "fixture-hash-v2",
};
export const fixtureArtifact: ArtifactDetail = {
  artifact: {
    public_id: "fixture-artifact",
    project_id: "fixture-project",
    artifact_type: "specification",
    title: fixtureArtifactVersion.title,
    status: "draft",
    current_version_id: fixtureArtifactVersion.public_id,
    version: 2,
    created_at: "2026-09-21T10:00:00Z",
    updated_at: "2026-09-21T11:00:00Z",
  },
  current_version: fixtureArtifactVersion,
};
export const fixtureOldArtifactVersion: ArtifactVersion = {
  ...fixtureArtifactVersion,
  public_id: "fixture-artifact-version-1",
  version: 1,
  title: "[FICTIF] Ancien titre",
  body_markdown: "# [FICTIF] Version initiale\nUne ancienne exigence.",
  content_hash: "fixture-hash-v1",
  sources: [],
  created_at: "2026-09-21T10:00:00Z",
};
export const fixtureDestinations: ArtifactDestinations = {
  items: [
    {
      artifact_type: "specification",
      provider: "notion",
      target_id: "fixture-notion-page",
      label: "[FICTIF] Documentation",
      origin: "company",
      revision: 0,
      project_id: null,
    },
  ],
};
