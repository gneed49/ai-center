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
export function typedDraft(value: Record<string, unknown>): TypedDraft | null {
  if (
    value.format !== "agent-artifact-v1" ||
    !value.draft ||
    typeof value.draft !== "object"
  )
    return null;
  const draft = value.draft as TypedDraft;
  return typeof draft.summary === "string" &&
    Array.isArray(draft.sections) &&
    Array.isArray(draft.tickets) &&
    Array.isArray(draft.open_questions)
    ? draft
    : null;
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
