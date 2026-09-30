// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ArtifactEditor } from "./artifact-editor";
import { draftMarkdown, typedDraft, type TypedDraft } from "./typed-draft";
import { prepareEditedDraft } from "./draft-edit-validation";
import type { ArtifactContent, ArtifactType } from "@/api/artifact-types";
afterEach(cleanup);
const id = (n: number) =>
  `10000000-0000-4000-8000-${n.toString().padStart(12, "0")}`;
function content(
  count = 1,
  kind: ArtifactType = "product_tickets",
): ArtifactContent {
  const draft: TypedDraft = {
    title: "[FICTIF] Travail produit",
    summary: "[FICTIF] Préparer les accès",
    sections: ["objective", "prioritization"].map((key) => ({
      key,
      title: key,
      body: "[FICTIF] Section conservée",
      source_ids: [id(80)],
    })),
    tickets: Array.from({ length: count }, (_, i) => ({
      title: `[FICTIF] Ticket ${i + 1}`,
      description: `Description ${i + 1}`,
      acceptance_criteria: [`Critère ${i + 1}`],
      source_ids: [id(i + 1)],
    })),
    open_questions: [],
  };
  return {
    title: draft.title,
    body_markdown: draftMarkdown(draft),
    structured_content: {
      format: "agent-artifact-v1",
      artifact_type: kind,
      draft,
    },
    sources: [id(80), ...draft.tickets.flatMap((t) => t.source_ids)].map(
      (public_id) => ({ kind: "knowledge", public_id }),
    ),
  };
}
function setup(
  initial = content(),
  artifactType: ArtifactType = "product_tickets",
  busy = false,
) {
  const onSubmit = vi.fn();
  const view = render(
    <ArtifactEditor
      initial={initial}
      artifactType={artifactType}
      busy={busy}
      onSubmit={onSubmit}
      submitLabel="Enregistrer"
    />,
  );
  return { onSubmit, ...view };
}
const input = (name: string) =>
  screen.getByRole("textbox", { name }) as HTMLInputElement;
function fill(index: number, title = `[FICTIF] Manuel ${index}`) {
  fireEvent.change(input(`Titre du ticket ${index}`), {
    target: { value: title },
  });
  fireEvent.change(input(`Description du ticket ${index}`), {
    target: { value: `Description manuelle ${index}` },
  });
  fireEvent.change(
    input(`Critères d’acceptation du ticket ${index} · un par ligne`),
    { target: { value: "Critère A\n\n  \nCritère B" } },
  );
}
it("adds a manually authored ticket without inventing citations and preserves version sources and canonical Markdown", () => {
  const initial = content(),
    snapshot = structuredClone(initial),
    { onSubmit } = setup(initial);
  expect(
    (
      screen.getByRole("button", {
        name: "Retirer le ticket 1",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Ajouter un ticket" }));
  expect(document.activeElement).toBe(input("Titre du ticket 2"));
  expect(onSubmit).not.toHaveBeenCalled();
  fill(2);
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  const sent: ArtifactContent = onSubmit.mock.calls[0][0],
    draft = typedDraft(sent.structured_content)!;
  expect(draft.tickets[1].source_ids).toEqual([]);
  expect(draft.tickets[0]).toEqual(
    typedDraft(initial.structured_content)!.tickets[0],
  );
  expect(draft.sections).toEqual(
    typedDraft(initial.structured_content)!.sections,
  );
  expect(draft.tickets[1].acceptance_criteria).toEqual([
    "Critère A",
    "Critère B",
  ]);
  expect(sent.sources).toEqual(initial.sources);
  expect(sent.body_markdown).toBe(draftMarkdown(draft));
  expect(initial).toEqual(snapshot);
});
it("removes the middle entry with stable input identity and restores it without overwriting another edit", () => {
  const initial = content(3),
    { onSubmit } = setup(initial),
    third = input("Titre du ticket 3");
  fireEvent.click(screen.getByRole("button", { name: "Retirer le ticket 2" }));
  expect(input("Titre du ticket 2")).toBe(third);
  expect(document.activeElement).toBe(third);
  fireEvent.change(third, { target: { value: "[FICTIF] Troisième modifié" } });
  fireEvent.click(screen.getByRole("button", { name: "Annuler le retrait" }));
  expect(document.activeElement).toBe(input("Titre du ticket 2"));
  expect(input("Titre du ticket 3")).toBe(third);
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  const draft = typedDraft(onSubmit.mock.calls[0][0].structured_content)!;
  expect(draft.tickets.map((t) => t.source_ids)).toEqual([
    [id(1)],
    [id(2)],
    [id(3)],
  ]);
  expect(draft.tickets[1]).toEqual(
    typedDraft(initial.structured_content)!.tickets[1],
  );
  expect(draft.tickets[2].title).toBe("[FICTIF] Troisième modifié");
});
it("preserves an existing multiline criterion and question while adding another ticket", () => {
  const initial = content(),
    original = typedDraft(initial.structured_content)!,
    multiline = Array.from(
      { length: 21 },
      (_, index) => `Étape ${index + 1}`,
    ).join("\n");
  original.tickets[0].acceptance_criteria = [multiline];
  original.open_questions = ["[FICTIF] Question existante\navec sa précision"];
  initial.structured_content = {
    ...initial.structured_content,
    draft: original,
  };
  initial.body_markdown = draftMarkdown(original);
  const { onSubmit } = setup(initial);
  fireEvent.click(screen.getByRole("button", { name: "Ajouter un ticket" }));
  fill(2);
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  expect(onSubmit).toHaveBeenCalledOnce();
  const saved = typedDraft(onSubmit.mock.calls[0][0].structured_content)!;
  expect(saved.tickets[0]).toEqual(original.tickets[0]);
  expect(saved.open_questions).toEqual(original.open_questions);
  expect(saved.tickets[1].acceptance_criteria).toEqual([
    "Critère A",
    "Critère B",
  ]);
});
it("enforces 30 entries, retains at least one, and expires undo on an addition or save", () => {
  const { onSubmit } = setup(content(29));
  fireEvent.click(screen.getByRole("button", { name: "Ajouter un ticket" }));
  expect(document.activeElement).toBe(input("Titre du ticket 30"));
  expect(
    (
      screen.getByRole("button", {
        name: "Ajouter un ticket",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Retirer le ticket 30" }));
  expect(document.activeElement).toBe(input("Titre du ticket 29"));
  fireEvent.click(screen.getByRole("button", { name: "Ajouter un ticket" }));
  expect(
    screen.queryByRole("button", { name: "Annuler le retrait" }),
  ).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Retirer le ticket 30" }));
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  expect(onSubmit).toHaveBeenCalledTimes(1);
  expect(
    screen.queryByRole("button", { name: "Annuler le retrait" }),
  ).toBeNull();
});
it.each(["kickoff", "specification", "technical_plan"] as const)(
  "does not infer ticket-count commands from a tickets array for %s",
  (kind) => {
    setup(content(2, kind), kind);
    expect(
      screen.queryByRole("button", { name: "Ajouter un ticket" }),
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Retirer le ticket 1" }),
    ).toBeNull();
  },
);
it("disables count operations and fields while saving", () => {
  setup(content(2), "product_tickets", true);
  expect(input("Titre du ticket 1").matches(":disabled")).toBe(true);
  for (const name of [
    "Ajouter un ticket",
    "Retirer le ticket 1",
    "Enregistrement…",
  ])
    expect(
      (screen.getByRole("button", { name }) as HTMLButtonElement).matches(
        ":disabled",
      ),
    ).toBe(true);
});
it.each([
  ["Titre du ticket 1", "é".repeat(101), "200 octets"],
  ["Description du ticket 1", "😀".repeat(2001), "8 000 octets"],
  [
    "Critères d’acceptation du ticket 1 · un par ligne",
    Array(21).fill("Critère").join("\n"),
    "entre 1 et 20",
  ],
  [
    "Critères d’acceptation du ticket 1 · un par ligne",
    "😀".repeat(501),
    "2 000 octets",
  ],
  [
    "Critères d’acceptation du ticket 1 · un par ligne",
    "\n  \n",
    "entre 1 et 20",
  ],
])(
  "keeps invalid %s and associates its UTF-8/list error",
  (name, value, expected) => {
    const { onSubmit } = setup();
    fireEvent.change(input(name), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(onSubmit).not.toHaveBeenCalled();
    expect(input(name).value).toBe(value);
    expect(input(name).getAttribute("aria-invalid")).toBe("true");
    expect(
      document.getElementById(input(name).getAttribute("aria-describedby")!)
        ?.textContent,
    ).toContain(expected);
    expect(document.activeElement).toBe(input(name));
  },
);
it("keeps a blank new entry editable while explaining its required fields", () => {
  const { onSubmit } = setup();
  fireEvent.click(screen.getByRole("button", { name: "Ajouter un ticket" }));
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  expect(onSubmit).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(input("Titre du ticket 2"));
  expect(screen.getByText("Le titre du ticket 2 est requis.")).toBeDefined();
});
it("rejects the aggregate UTF-8 draft size even when individual entries are valid", () => {
  const initial = content(20),
    draft = typedDraft(initial.structured_content)!;
  draft.tickets = draft.tickets.map((t) => ({
    ...t,
    description: "a".repeat(7000),
  }));
  const result = prepareEditedDraft(
    draft,
    initial.title,
    initial,
    "product_tickets",
  );
  expect(result.errors.document).toContain("taille autorisée");
  expect(Object.keys(result.errors)).toEqual(["document"]);
});
