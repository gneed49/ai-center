import { request } from "./client";
import type {
  Publication,
  PublicationDetail,
  Publications,
  PublishArtifact,
  SaveWorkToolConnection,
  WorkToolConnection,
  WorkToolSettings,
  WorkToolTest,
} from "./work-tool-types";
const id = encodeURIComponent;
function command(input: unknown, key: string): RequestInit {
  return {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}
export const workToolsApi = {
  settings: () => request<WorkToolSettings>("/api/work-tools"),
  save: (input: SaveWorkToolConnection, key: string) =>
    request<WorkToolConnection>(
      "/api/work-tools/connections",
      command(input, key),
    ),
  disable: (connectionId: string, revision: number, key: string) =>
    request<WorkToolConnection>(
      `/api/work-tools/connections/${id(connectionId)}/disable`,
      command({ expected_revision: revision }, key),
    ),
  test: (connectionId: string, key: string) =>
    request<WorkToolTest>(
      `/api/work-tools/connections/${id(connectionId)}/test`,
      command({}, key),
    ),
  publications: (artifactId: string, offset = 0) =>
    request<Publications>(
      `/api/artifacts/${id(artifactId)}/publications?limit=10&offset=${offset}`,
    ),
  publish: (artifactId: string, input: PublishArtifact, key: string) =>
    request<Publication>(
      `/api/artifacts/${id(artifactId)}/publications`,
      command(input, key),
    ),
  publication: (publicationId: string) =>
    request<PublicationDetail>(`/api/publications/${id(publicationId)}`),
  reconcile: (publicationId: string, externalId: string, key: string) =>
    request<Publication>(
      `/api/publications/${id(publicationId)}/reconcile`,
      command({ external_id: externalId }, key),
    ),
  refresh: (publicationId: string, key: string) =>
    request<Publication>(
      `/api/publications/${id(publicationId)}/refresh`,
      command({}, key),
    ),
  cancel: (publicationId: string, key: string) =>
    request<Publication>(
      `/api/publications/${id(publicationId)}/cancel`,
      command({}, key),
    ),
};
