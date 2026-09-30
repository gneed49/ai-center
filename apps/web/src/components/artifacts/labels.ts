import type { ArtifactType, DestinationProvider } from "@/api/artifact-types";

export const artifactLabels: Record<ArtifactType, string> = {
  kickoff: "Note de lancement",
  specification: "Spécification",
  product_tickets: "Tickets produit",
  technical_plan: "Plan technique",
  technical_tickets: "Tickets techniques",
};
export const providerLabels: Record<DestinationProvider, string> = {
  internal: "AI Center",
  notion: "Notion",
  linear: "Linear",
  github: "GitHub",
};
