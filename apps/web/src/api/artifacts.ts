import { request, requestText } from "./client";
import type {
  ArtifactDetail,
  ArtifactDestinations,
  ArtifactFilters,
  ArtifactPage,
  ArtifactSummary,
  ArtifactType,
  ArtifactVersion,
  CreateArtifact,
  SaveArtifact,
  SetArtifactDestination,
} from "./artifact-types";

const id = encodeURIComponent;
function command(input: unknown, key: string): RequestInit {
  return {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}
function query(filters: ArtifactFilters = {}) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters))
    if (value !== undefined && value !== "") params.set(key, String(value));
  return params.size ? `?${params}` : "";
}
function destinationPath(projectId?: string) {
  return projectId
    ? `/api/projects/${id(projectId)}/artifact-destinations`
    : "/api/artifact-destinations";
}

export const artifactsApi = {
  list: (projectId: string, filters?: ArtifactFilters) =>
    request<ArtifactPage<ArtifactSummary>>(
      `/api/projects/${id(projectId)}/artifacts${query(filters)}`,
    ),
  create: (projectId: string, input: CreateArtifact, key: string) =>
    request<ArtifactDetail>(
      `/api/projects/${id(projectId)}/artifacts`,
      command(input, key),
    ),
  generate: (
    projectId: string,
    input: {
      session_id: string;
      artifact_type: ArtifactType;
      instructions: string;
    },
    key: string,
  ) =>
    request<ArtifactDetail>(
      `/api/projects/${id(projectId)}/artifacts/generate`,
      command(input, key),
    ),
  generationReceipt: (projectId: string, key: string) =>
    request<{
      status: string;
      can_retry: boolean;
      result: ArtifactDetail | null;
    }>(`/api/projects/${id(projectId)}/artifact-generations/${id(key)}`, {
      cache: "no-store",
    }),
  fromDeliverable: (projectId: string, deliverableId: string, key: string) =>
    request<ArtifactDetail>(
      `/api/projects/${id(projectId)}/deliverables/${id(deliverableId)}/artifact`,
      command({}, key),
    ),
  conversionReceipt: (projectId: string, key: string) =>
    request<{
      status: string;
      can_retry: boolean;
      result: ArtifactDetail | null;
    }>(`/api/projects/${id(projectId)}/artifact-conversions/${id(key)}`, {
      cache: "no-store",
    }),
  detail: (artifactId: string) =>
    request<ArtifactDetail>(`/api/artifacts/${id(artifactId)}`),
  save: (artifactId: string, input: SaveArtifact, key: string) =>
    request<ArtifactDetail>(
      `/api/artifacts/${id(artifactId)}/draft`,
      command(input, key),
    ),
  validate: (artifactId: string, versionId: string, key: string) =>
    request<ArtifactDetail>(
      `/api/artifacts/${id(artifactId)}/validate`,
      command({ expected_version_id: versionId }, key),
    ),
  versions: (
    artifactId: string,
    filters?: Pick<ArtifactFilters, "limit" | "offset">,
  ) =>
    request<ArtifactPage<ArtifactVersion>>(
      `/api/artifacts/${id(artifactId)}/versions${query(filters)}`,
    ),
  version: (artifactId: string, versionId: string) =>
    request<ArtifactVersion>(
      `/api/artifacts/${id(artifactId)}/versions/${id(versionId)}`,
    ),
  export: (
    artifactId: string,
    versionId: string,
    format: "markdown" | "json",
  ) =>
    requestText(
      `/api/artifacts/${id(artifactId)}/versions/${id(versionId)}/export?format=${format}`,
      { cache: "no-store" },
    ),
  destinations: (projectId?: string) =>
    request<ArtifactDestinations>(destinationPath(projectId)),
  setDestination: (
    projectId: string | undefined,
    input: SetArtifactDestination,
    key: string,
  ) =>
    request<ArtifactDestinations>(
      destinationPath(projectId),
      command(input, key),
    ),
  resetDestination: (
    projectId: string | undefined,
    type: ArtifactType,
    revision: number,
    key: string,
  ) =>
    request<ArtifactDestinations>(
      `${destinationPath(projectId)}/reset`,
      command({ artifact_type: type, expected_revision: revision }, key),
    ),
};
