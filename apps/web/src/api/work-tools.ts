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
import type {
  PreviewTickets,
  PublishTickets,
  TicketCoverage,
  TicketPreview,
  TicketPublicationResult,
  TicketPublicationReceipt,
} from "./ticket-publication-types";
const id = encodeURIComponent;
async function hydratePublication(value: Publication): Promise<Publication> {
  const normalized = {
    ...value,
    source_ticket_index: value.source_ticket_index ?? -1,
  };
  if (value.title != null && value.source_version_number != null)
    return normalized;
  const detail = await request<PublicationDetail>(
    `/api/publications/${id(value.public_id)}`,
    { cache: "no-store" },
  );
  return {
    ...normalized,
    title: detail.publication.title,
    source_version_number: detail.publication.source_version_number,
  };
}
function command(input: unknown, key: string): RequestInit {
  return {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}
export const workToolsApi = {
  ticketCoverage: (
    artifactId: string,
    versionId: string,
    provider: string,
    targetId: string,
  ) =>
    request<TicketCoverage>(
      `/api/artifacts/${id(artifactId)}/ticket-publications?${new URLSearchParams({ version_id: versionId, provider, target_id: targetId })}`,
      { cache: "no-store" },
    ),
  ticketPreview: (artifactId: string, input: PreviewTickets) =>
    request<TicketPreview>(
      `/api/artifacts/${id(artifactId)}/ticket-publications/preview`,
      { method: "POST", body: JSON.stringify(input) },
    ),
  publishTickets: (artifactId: string, input: PublishTickets, key: string) =>
    request<TicketPublicationResult>(
      `/api/artifacts/${id(artifactId)}/ticket-publications`,
      command(input, key),
    ),
  ticketReceipt: (artifactId: string, key: string) =>
    request<TicketPublicationReceipt>(
      `/api/artifacts/${id(artifactId)}/ticket-publication-commands/${id(key)}`,
      { cache: "no-store" },
    ),
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
    ).then(hydratePublication),
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
