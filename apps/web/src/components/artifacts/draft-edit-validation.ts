import type { ArtifactContent, ArtifactType } from "@/api/artifact-types";
import { draftMarkdown, type TypedDraft } from "./typed-draft";
export type DraftErrors = Record<string, string>;
const bytes = (value: string) => new TextEncoder().encode(value).length;
const lines = (values: string[]) =>
  values.filter((value) => value.trim().length > 0);
export function prepareEditedDraft(
  draft: TypedDraft,
  title: string,
  initial: ArtifactContent,
  kind?: ArtifactType,
): { content: ArtifactContent; errors: DraftErrors } {
  const normalized: TypedDraft = {
    ...draft,
    title: title.trim(),
    tickets: draft.tickets.map((ticket) => ({
      ...ticket,
      acceptance_criteria: lines(ticket.acceptance_criteria),
    })),
    open_questions: lines(draft.open_questions),
  };
  const errors: DraftErrors = {};
  function bounded(key: string, value: string, label: string, max: number) {
    if (!value.trim()) errors[key] = `${label} est requis.`;
    else if (bytes(value) > max)
      errors[key] =
        `${label} dépasse ${max.toLocaleString("fr-FR")} octets UTF-8. Raccourcissez ce texte.`;
  }
  bounded("title", normalized.title, "Le titre du livrable", 200);
  bounded("summary", normalized.summary, "Le résumé", 4000);
  const allowed = new Set(
    initial.sources
      .filter((s) =>
        [
          "knowledge",
          "artifact_version",
          "tool_source_observation",
          "publication_observation",
        ].includes(s.kind),
      )
      .map((s) => s.public_id),
  );
  const cited = new Set<string>();
  function sources(ids: string[]) {
    ids.forEach((id) => cited.add(id));
    if (
      ids.length > 100 ||
      new Set(ids).size !== ids.length ||
      ids.some((id) => !allowed.has(id))
    )
      errors.document =
        "Les citations conservées ne correspondent plus aux sources de cette version. Rouvrez le document ou faites vérifier ses sources.";
  }
  normalized.sections.forEach((section, i) => {
    bounded(
      `section.${i}`,
      section.body,
      `Le contenu de la section ${i + 1}`,
      12000,
    );
    if (!section.title.trim() || bytes(section.title) > 200)
      errors.document =
        "Le titre d’une section conservée dépasse les limites du document.";
    sources(section.source_ids);
  });
  if (
    (kind === "product_tickets" || kind === "technical_tickets") &&
    (normalized.tickets.length < 1 || normalized.tickets.length > 30)
  )
    errors.document = "Le livrable doit contenir entre 1 et 30 tickets.";
  normalized.tickets.forEach((ticket, i) => {
    bounded(
      `ticket.${i}.title`,
      ticket.title,
      `Le titre du ticket ${i + 1}`,
      200,
    );
    bounded(
      `ticket.${i}.description`,
      ticket.description,
      `La description du ticket ${i + 1}`,
      8000,
    );
    const key = `ticket.${i}.criteria`;
    if (
      !ticket.acceptance_criteria.length ||
      ticket.acceptance_criteria.length > 20
    )
      errors[key] =
        `Le ticket ${i + 1} doit avoir entre 1 et 20 critères d’acceptation, un par ligne.`;
    else if (ticket.acceptance_criteria.some((value) => bytes(value) > 2000))
      errors[key] =
        `Un critère du ticket ${i + 1} dépasse 2 000 octets UTF-8. Raccourcissez cette ligne.`;
    sources(ticket.source_ids);
  });
  if (
    normalized.open_questions.length > 30 ||
    normalized.open_questions.some((value) => bytes(value) > 2000)
  )
    errors.questions =
      "Conservez au plus 30 points à clarifier, chacun limité à 2 000 octets UTF-8.";
  const content: ArtifactContent = {
    title: normalized.title,
    body_markdown: draftMarkdown(normalized),
    structured_content: { ...initial.structured_content, draft: normalized },
    sources: initial.sources.map(({ kind, public_id }) => ({
      kind,
      public_id,
    })),
  };
  if (cited.size > 98 || initial.sources.length > 100)
    errors.document =
      "Le brouillon dépasse les limites de sources : 98 citations distinctes et 100 sources de version.";
  if (
    bytes(JSON.stringify(normalized)) > 100_000 ||
    bytes(JSON.stringify(content.structured_content)) > 131_072 ||
    bytes(content.body_markdown) > 262_144
  )
    errors.document =
      "Le document dépasse la taille autorisée. Réduisez son contenu ou répartissez le travail entre plusieurs livrables.";
  return { content, errors };
}
