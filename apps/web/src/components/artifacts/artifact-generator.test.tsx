// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import type { ArtifactType } from "@/api/artifact-types";
import { setRequestIdentity } from "@/api/request-context";
import { fixtureSession } from "@/test-fixtures/context-loop";
import { ArtifactGenerator } from "./artifact-generator";
import { ArtifactEditor } from "./artifact-editor";
vi.mock("@/api/artifacts", () => ({
  artifactsApi: { generate: vi.fn(), generationReceipt: vi.fn() },
}));
beforeEach(() => {
  localStorage.clear();
  setRequestIdentity("fixture-actor", "fixture-workspace", null);
  vi.mocked(artifactsApi.generationReceipt).mockResolvedValue({
    status: "interrupted",
    can_retry: true,
    result: null,
  });
});

it("keeps unsent text across type changes while restoring each type's exact command", async () => {
  vi.mocked(artifactsApi.generate).mockRejectedValue(
    new Error("[FICTIF] Réponse perdue"),
  );
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const view = (type: ArtifactType) => (
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <ArtifactGenerator
          projectId="fixture-project"
          type={type}
          sessions={[fixtureSession.session]}
          selectedSessionId={fixtureSession.session.public_id}
        />
      </MemoryRouter>
    </QueryClientProvider>
  );
  const rendered = render(view("specification"));
  fireEvent.change(screen.getByLabelText("Ce que le document doit préparer"), {
    target: { value: "[FICTIF] Première demande" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Générer le brouillon avec l’agent" }),
  );
  await screen.findByText("Le brouillon n’a pas pu être confirmé");
  const first = vi.mocked(artifactsApi.generate).mock.calls[0];
  rendered.rerender(view("product_tickets"));
  expect(
    (
      screen.getByLabelText(
        "Ce que le document doit préparer",
      ) as HTMLTextAreaElement
    ).value,
  ).toBe("[FICTIF] Première demande");
  fireEvent.change(screen.getByLabelText("Ce que le document doit préparer"), {
    target: { value: "[FICTIF] Tickets à préparer" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Générer le brouillon avec l’agent" }),
  );
  await screen.findByText("Le brouillon n’a pas pu être confirmé");
  expect(artifactsApi.generate).toHaveBeenCalledTimes(2);
  expect(vi.mocked(artifactsApi.generate).mock.calls[1][1]).toMatchObject({
    artifact_type: "product_tickets",
    instructions: "[FICTIF] Tickets à préparer",
  });
  expect(vi.mocked(artifactsApi.generate).mock.calls[1][2]).not.toBe(first[2]);
  rendered.rerender(view("specification"));
  const restored = screen.getByLabelText(
    "Ce que le document doit préparer",
  ) as HTMLTextAreaElement;
  expect(restored.value).toBe("[FICTIF] Première demande");
  expect(restored.disabled).toBe(true);
  await waitFor(() =>
    expect(artifactsApi.generationReceipt).toHaveBeenCalledWith(
      "fixture-project",
      first[2],
    ),
  );
  expect(artifactsApi.generate).toHaveBeenCalledTimes(2);
  fireEvent.click(
    screen.getByRole("button", { name: "Préparer une nouvelle demande" }),
  );
  expect(restored.value).toBe("[FICTIF] Première demande");
  expect(restored.disabled).toBe(false);
});

it.each(["actor", "workspace", "project"])(
  "does not carry an unsent draft into another %s",
  (boundary) => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const view = (projectId: string) => (
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <ArtifactGenerator
            projectId={projectId}
            type="specification"
            sessions={[fixtureSession.session]}
            selectedSessionId={fixtureSession.session.public_id}
          />
        </MemoryRouter>
      </QueryClientProvider>
    );
    const rendered = render(view("fixture-project"));
    fireEvent.change(
      screen.getByLabelText("Ce que le document doit préparer"),
      { target: { value: "[FICTIF] Saisie du contexte précédent" } },
    );
    setRequestIdentity(
      boundary === "actor" ? "fixture-other-actor" : "fixture-actor",
      boundary === "workspace"
        ? "fixture-other-workspace"
        : "fixture-workspace",
      null,
    );
    rendered.rerender(
      view(
        boundary === "project" ? "fixture-other-project" : "fixture-project",
      ),
    );
    expect(
      (
        screen.getByLabelText(
          "Ce que le document doit préparer",
        ) as HTMLTextAreaElement
      ).value,
    ).toBe("");
    expect(artifactsApi.generate).not.toHaveBeenCalled();
  },
);
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
it("generates typed tickets from the chosen conversation and reuses the command after a lost response", async () => {
  vi.mocked(artifactsApi.generate).mockRejectedValue(new Error("Interrompu"));
  render(
    <QueryClientProvider
      client={
        new QueryClient({
          defaultOptions: {
            mutations: { retry: false },
            queries: { retry: false },
          },
        })
      }
    >
      <MemoryRouter>
        <Routes>
          <Route
            path="*"
            element={
              <ArtifactGenerator
                projectId="project"
                type="product_tickets"
                sessions={[fixtureSession.session]}
                selectedSessionId={fixtureSession.session.public_id}
              />
            }
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
  fireEvent.change(screen.getByLabelText("Ce que le document doit préparer"), {
    target: { value: "[FICTIF] Découper les accès" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Générer le brouillon avec l’agent" }),
  );
  await screen.findByText("Le brouillon n’a pas pu être confirmé");
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Reprendre la même demande",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Reprendre la même demande" }),
  );
  await waitFor(() => expect(artifactsApi.generate).toHaveBeenCalledTimes(2));
  expect(vi.mocked(artifactsApi.generate).mock.calls[0]).toEqual(
    vi.mocked(artifactsApi.generate).mock.calls[1],
  );
  expect(artifactsApi.generate).toHaveBeenCalledWith(
    "project",
    {
      artifact_type: "product_tickets",
      session_id: fixtureSession.session.public_id,
      instructions: "[FICTIF] Découper les accès",
    },
    expect.any(String),
  );
});
it("keeps typed sections and outgoing Markdown synchronized while editing", () => {
  const submit = vi.fn();
  const draft = {
    title: "[FICTIF] Spec",
    summary: "[FICTIF] Résumé",
    sections: [
      {
        key: "problem",
        title: "Problème",
        body: "Ancien texte",
        source_ids: ["source"],
      },
    ],
    tickets: [],
    open_questions: [],
  };
  render(
    <ArtifactEditor
      initial={{
        title: draft.title,
        body_markdown: "Ancien texte",
        structured_content: {
          format: "agent-artifact-v1",
          artifact_type: "specification",
          draft,
        },
        sources: [{ kind: "knowledge", public_id: "source" }],
      }}
      busy={false}
      submitLabel="Enregistrer"
      onSubmit={submit}
    />,
  );
  fireEvent.change(screen.getByLabelText("Problème"), {
    target: { value: "[FICTIF] Correction relue" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  expect(
    submit.mock.calls[0][0].structured_content.draft.sections[0].body,
  ).toBe("[FICTIF] Correction relue");
  expect(submit.mock.calls[0][0].body_markdown).toContain(
    "[FICTIF] Correction relue",
  );
});

it("restores a pending command after reload and only polls its receipt", async () => {
  vi.mocked(artifactsApi.generate).mockImplementation(
    () => new Promise(() => {}),
  );
  vi.mocked(artifactsApi.generationReceipt).mockResolvedValue({
    status: "processing",
    can_retry: false,
    result: null,
  });
  function open() {
    return render(
      <QueryClientProvider
        client={
          new QueryClient({
            defaultOptions: {
              queries: { retry: false },
              mutations: { retry: false },
            },
          })
        }
      >
        <MemoryRouter>
          <ArtifactGenerator
            projectId="reload-project"
            type="specification"
            sessions={[fixtureSession.session]}
            selectedSessionId={fixtureSession.session.public_id}
          />
        </MemoryRouter>
      </QueryClientProvider>,
    );
  }
  const first = open();
  fireEvent.change(screen.getByLabelText("Ce que le document doit préparer"), {
    target: { value: "[FICTIF] Conserver cette consigne après rechargement" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Générer le brouillon avec l’agent" }),
  );
  await waitFor(() => expect(artifactsApi.generate).toHaveBeenCalledTimes(1));
  const key = vi.mocked(artifactsApi.generate).mock.calls[0][2];
  first.unmount();
  open();
  await screen.findByText(/Le serveur prépare votre brouillon/);
  expect(
    (
      screen.getByLabelText(
        "Ce que le document doit préparer",
      ) as HTMLTextAreaElement
    ).value,
  ).toContain("après rechargement");
  expect(artifactsApi.generationReceipt).toHaveBeenCalledWith(
    "reload-project",
    key,
  );
  expect(artifactsApi.generate).toHaveBeenCalledTimes(1);
  expect(
    (
      screen.getByRole("button", {
        name: "Reprendre la même demande",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
});
