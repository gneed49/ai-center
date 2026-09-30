// @vitest-environment jsdom
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { fixtureArtifactVersion } from "@/test-fixtures/artifacts";
import { draftMarkdown } from "./typed-draft";
import { TypedDraftView } from "./typed-draft-view";

afterEach(cleanup);
const draft = {
  title: "[FICTIF] Document",
  summary: "Résumé lisible",
  sections: [
    {
      key: "objective",
      title: "objective",
      body: "Objectif métier",
      source_ids: [],
    },
  ],
  tickets: Array.from({ length: 30 }, (_, i) => ({
    title: `Travail ${i + 1}`,
    description: "Description exacte",
    acceptance_criteria: ["Critère vérifiable"],
    source_ids: [],
  })),
  open_questions: ["Responsable à confirmer"],
};
it.each([
  "kickoff",
  "specification",
  "product_tickets",
  "technical_plan",
  "technical_tickets",
])(
  "reads %s with French headings and lists while preserving canonical export",
  (artifact_type) => {
    const original = draftMarkdown(draft);
    const version = {
      ...fixtureArtifactVersion,
      structured_content: { format: "agent-artifact-v1", artifact_type, draft },
      body_markdown: original,
    };
    render(<TypedDraftView version={version} ticket={null} />);
    expect(screen.getByRole("heading", { name: "Objectif" })).toBeDefined();
    expect(screen.getByRole("heading", { name: "Travail 30" })).toBeDefined();
    expect(screen.getAllByRole("listitem").length).toBe(31);
    expect(screen.queryByText(/^##/)).toBeNull();
    expect(version.body_markdown).toBe(original);
  },
);
it.each(["0", "29"])(
  "focuses exactly ticket %s in the supplied immutable version",
  (ticket) => {
    const version = {
      ...fixtureArtifactVersion,
      version: 1,
      structured_content: { format: "agent-artifact-v1", draft },
    };
    render(<TypedDraftView version={version} ticket={ticket} />);
    expect(document.activeElement).toBe(
      screen.getByRole("article", {
        name: `Ticket ${Number(ticket) + 1} · version 1`,
      }),
    );
  },
);
it.each(["30", "01", "-1", "1e0", "text"])(
  "rejects invalid ticket address %s without choosing another entry",
  (ticket) => {
    render(
      <TypedDraftView
        version={{
          ...fixtureArtifactVersion,
          structured_content: { format: "agent-artifact-v1", draft },
        }}
        ticket={ticket}
      />,
    );
    expect(screen.getByRole("status").textContent).toContain(
      "n’existe pas dans cette version",
    );
    expect(document.activeElement).toBe(document.body);
  },
);
