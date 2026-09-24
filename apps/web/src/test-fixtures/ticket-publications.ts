import type {
  PreviewTickets,
  TicketCoverage,
  TicketPreview,
  TicketPublicationResult,
} from "@/api/ticket-publication-types";
import { fixtureArtifactVersion } from "./artifacts";
import { fixturePublication, fixtureTools } from "./work-tools";
import type { ArtifactVersion } from "@/api/artifact-types";
export const fixtureTicketVersion: ArtifactVersion = {
  ...fixtureArtifactVersion,
  status: "validated",
  validated_at: "2026-09-24T10:00:00Z",
  structured_content: {
    format: "agent-artifact-v1",
    artifact_type: "product_tickets",
    draft: {
      title: "[FICTIF] Tickets",
      summary: "[FICTIF] Travaux",
      sections: [],
      open_questions: [],
      tickets: Array.from({ length: 30 }, (_, i) => ({
        title: `[FICTIF] Travail ${i + 1}`,
        description: `Description ${i + 1}`,
        acceptance_criteria: [`Résultat ${i + 1} vérifié`],
        source_ids: [],
      })),
    },
  },
};
export const fixtureTicketTools = {
  ...fixtureTools,
  connections: [
    {
      ...fixtureTools.connections[0],
      provider: "linear" as const,
      name: "[FICTIF] Équipe produit",
    },
  ],
  capabilities: [
    {
      provider: "linear" as const,
      create: true,
      read: true,
      reconcile: true,
      update: false,
    },
  ],
};
export const fixtureTicketTarget = "11111111-1111-4111-8111-111111111111";
export const fixtureTicketCoverage: TicketCoverage = {
  artifact_id: "fixture-artifact",
  version_id: fixtureTicketVersion.public_id,
  source_version_number: 2,
  provider: "linear",
  target_id: fixtureTicketTarget,
  items: Array.from({ length: 30 }, (_, i) => ({
    source_ticket_index: i,
    title: `[FICTIF] Travail ${i + 1}`,
    existing_publication: null,
  })),
};
export function fixtureTicketPreview(input: PreviewTickets): TicketPreview {
  return {
    ...fixtureTicketCoverage,
    connection_id: input.connection_id,
    connection_revision: 2,
    content_hash: "f".repeat(64),
    ticket_indexes: input.ticket_indexes,
    requested_count: input.ticket_indexes.length,
    new_count: input.ticket_indexes.length,
    existing_count: 0,
    items: input.ticket_indexes.map((i) => ({
      ...fixtureTicketCoverage.items[i],
      business_body_markdown: `Description ${i + 1}\nCritères : Résultat ${i + 1} vérifié`,
    })),
    prior_publications: [],
    prior_publications_fingerprint: "a".repeat(64),
    preview_fingerprint: "b".repeat(64),
    requires_additional_confirmation: false,
    capacity: {
      available_pending: 25,
      available_hourly: 100,
      max_pending: 25,
      max_per_hour: 100,
    },
  };
}
export const fixtureTicketResult: TicketPublicationResult = {
  artifact_id: "fixture-artifact",
  version_id: fixtureTicketVersion.public_id,
  source_version_number: 2,
  provider: "linear",
  target_id: fixtureTicketTarget,
  created_count: 3,
  existing_count: 0,
  publications: (["succeeded", "needs_review", "failed"] as const).map(
    (status, i) => ({
      ...fixturePublication,
      public_id: `fixture-ticket-publication-${i}`,
      provider: "linear",
      target_id: fixtureTicketTarget,
      source_ticket_index: i,
      title: `[FICTIF] Travail ${i + 1}`,
      source_version_number: 2,
      status,
      external_url:
        status === "succeeded" ? "https://linear.app/fixture/issue/F-1" : null,
    }),
  ),
};
