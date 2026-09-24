export interface TypedDraft {
  title: string;
  summary: string;
  sections: {
    key: string;
    title: string;
    body: string;
    source_ids: string[];
  }[];
  tickets: {
    title: string;
    description: string;
    acceptance_criteria: string[];
    source_ids: string[];
  }[];
  open_questions: string[];
}
export const sectionLabels: Record<string, string> = {
  objective: "Objectif",
  scope: "Périmètre",
  stakeholders: "Parties prenantes",
  milestones: "Étapes",
  risks: "Risques",
  problem: "Problème",
  requirements: "Exigences",
  business_rules: "Règles métier",
  acceptance_criteria: "Critères d’acceptation",
  out_of_scope: "Hors périmètre",
  prioritization: "Priorisation",
  architecture: "Architecture",
  delivery: "Livraison",
  dependencies: "Dépendances",
  validation: "Validation",
};
export function sectionLabel(key: string, fallback: string) {
  return sectionLabels[key] ?? fallback;
}
export function ticketIndex(value: string | null): number | null {
  return value !== null && /^(?:[0-9]|[12][0-9])$/.test(value)
    ? Number(value)
    : null;
}
function strings(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.every((item) => typeof item === "string")
  );
}
function object(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
export function typedDraft(value: Record<string, unknown>): TypedDraft | null {
  if (value.format !== "agent-artifact-v1" || !object(value.draft)) return null;
  const draft = value.draft;
  if (
    typeof draft.title !== "string" ||
    typeof draft.summary !== "string" ||
    !strings(draft.open_questions) ||
    !Array.isArray(draft.sections) ||
    !Array.isArray(draft.tickets) ||
    draft.tickets.length > 30 ||
    !draft.sections.every(
      (s) =>
        object(s) &&
        typeof s.key === "string" &&
        typeof s.title === "string" &&
        typeof s.body === "string" &&
        strings(s.source_ids),
    ) ||
    !draft.tickets.every(
      (t) =>
        object(t) &&
        typeof t.title === "string" &&
        typeof t.description === "string" &&
        strings(t.acceptance_criteria) &&
        strings(t.source_ids),
    )
  )
    return null;
  return draft as unknown as TypedDraft;
}
export function draftMarkdown(draft: TypedDraft) {
  let body = `${draft.summary}\n`;
  for (const section of draft.sections)
    body += `\n## ${section.title}\n\n${section.body}\n`;
  for (const [index, ticket] of draft.tickets.entries()) {
    body += `\n## Ticket ${index + 1} — ${ticket.title}\n\n${ticket.description}\n\nCritères d’acceptation :\n`;
    for (const criterion of ticket.acceptance_criteria)
      body += `- ${criterion}\n`;
  }
  if (draft.open_questions.length) {
    body += "\n## Points à clarifier\n\n";
    for (const question of draft.open_questions) body += `- ${question}\n`;
  }
  return body;
}
