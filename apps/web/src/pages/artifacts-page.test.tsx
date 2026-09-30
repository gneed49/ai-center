// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { api, ApiError } from "@/api/client";
import { companyApi } from "@/api/company";
import { artifactsApi } from "@/api/artifacts";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { fixtureSession, fixtureSnapshot } from "@/test-fixtures/context-loop";
import { ArtifactsPage } from "./artifacts-page";

vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: { snapshot: vi.fn(), session: vi.fn() },
}));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/artifacts", () => ({
  artifactsApi: {
    list: vi.fn(),
    create: vi.fn(),
    generate: vi.fn(),
    generationReceipt: vi.fn(),
  },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(api.session).mockResolvedValue({
    ...fixtureSession,
    messages: [
      {
        public_id: "fixture-message",
        role: "assistant",
        agent_scope: "product",
        content: "[FICTIF] Texte proposé",
        created_at: "2026-09-21T11:00:00Z",
        metadata: {},
      },
    ],
  });
  vi.mocked(artifactsApi.list).mockResolvedValue({
    items: [],
    total: 0,
    limit: 25,
    offset: 0,
  });
  vi.mocked(artifactsApi.create).mockRejectedValue(
    new ApiError("Délai dépassé", 504),
  );
});
afterEach(() => {
  cleanup();
  client.clear();
});
function Destination() {
  return <p>{useLocation().pathname}</p>;
}
function openPage(path: string) {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/artifacts" element={<ArtifactsPage />} />
          <Route
            path="/projects/:projectId/artifacts"
            element={<ArtifactsPage />}
          />
          <Route path="*" element={<Destination />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("manual artifact creation", () => {
  it("does not create a manual draft with the previous type during navigation", async () => {
    client.setQueryData(["company"], fixtureCompany);
    client.setQueryData(["snapshot", "fixture-project"], fixtureSnapshot);
    openPage("/projects/fixture-project/artifacts?create=1");
    fireEvent.change(await screen.findByLabelText("Titre du livrable"), {
      target: { value: "[FICTIF] Brouillon manuel" },
    });
    fireEvent.change(screen.getByLabelText("Contenu"), {
      target: { value: "[FICTIF] Contenu conservé" },
    });
    const form = screen.getByRole("form", { name: "Édition du livrable" });
    await act(async () => {
      fireEvent.change(screen.getByLabelText("Type de livrable"), {
        target: { value: "product_tickets" },
      });
      fireEvent.submit(form);
    });
    expect(artifactsApi.create).not.toHaveBeenCalled();
    expect(
      (screen.getByLabelText("Contenu") as HTMLTextAreaElement).value,
    ).toBe("[FICTIF] Contenu conservé");
    fireEvent.submit(form);
    await waitFor(() => expect(artifactsApi.create).toHaveBeenCalledOnce());
    expect(vi.mocked(artifactsApi.create).mock.calls[0][1].artifact_type).toBe(
      "product_tickets",
    );
  });
  it("does not generate the previous type when submission immediately follows a type change", async () => {
    client.setQueryData(["company"], fixtureCompany);
    client.setQueryData(["snapshot", "fixture-project"], fixtureSnapshot);
    client.setQueryData(
      ["session", "fixture-project", "fixture-session"],
      fixtureSession,
    );
    vi.mocked(artifactsApi.generate).mockImplementation(
      () => new Promise(() => {}),
    );
    vi.mocked(artifactsApi.generationReceipt).mockResolvedValue({
      status: "processing",
      can_retry: false,
      result: null,
    });
    openPage(
      "/projects/fixture-project/artifacts?create=1&session=fixture-session",
    );
    fireEvent.change(
      await screen.findByLabelText("Ce que le document doit préparer"),
      { target: { value: "[FICTIF] Préparer le bon type" } },
    );
    const submit = screen.getByRole("button", {
      name: "Générer le brouillon avec l’agent",
    });
    await act(async () => {
      fireEvent.change(screen.getByLabelText("Type de livrable"), {
        target: { value: "product_tickets" },
      });
      fireEvent.click(submit);
    });
    expect(artifactsApi.generate).not.toHaveBeenCalled();
    fireEvent.click(
      screen.getByRole("button", { name: "Générer le brouillon avec l’agent" }),
    );
    await waitFor(() => expect(artifactsApi.generate).toHaveBeenCalledOnce());
    expect(
      vi.mocked(artifactsApi.generate).mock.calls[0][1].artifact_type,
    ).toBe("product_tickets");
  });
  it("keeps instructions entered while the selected artifact type is still updating", async () => {
    // Isolate the pending type navigation from unrelated initial source loading.
    client.setQueryData(["company"], fixtureCompany);
    client.setQueryData(["snapshot", "fixture-project"], fixtureSnapshot);
    client.setQueryData(
      ["session", "fixture-project", "fixture-session"],
      fixtureSession,
    );
    openPage(
      "/projects/fixture-project/artifacts?create=1&session=fixture-session",
    );
    const instructions = await screen.findByRole("textbox", {
      name: "Ce que le document doit préparer",
    });
    act(() => {
      fireEvent.change(screen.getByLabelText("Type de livrable"), {
        target: { value: "product_tickets" },
      });
      fireEvent.change(instructions, {
        target: {
          value: "[FICTIF] Consigne saisie pendant le changement de type",
        },
      });
    });
    expect(
      (screen.getByLabelText("Type de livrable") as HTMLSelectElement).value,
    ).toBe("product_tickets");
    expect(
      (
        screen.getByRole("textbox", {
          name: "Ce que le document doit préparer",
        }) as HTMLTextAreaElement
      ).value,
    ).toBe("[FICTIF] Consigne saisie pendant le changement de type");
  });
  it("starts from the selected company conversation without copying text automatically and retries the same command", async () => {
    openPage(
      "/artifacts?project=fixture-company-scope&create=1&session=fixture-session",
    );
    const body = await screen.findByRole("textbox", { name: "Contenu" });
    expect((body as HTMLTextAreaElement).value).toBe("");
    expect(api.session).toHaveBeenCalledWith(
      "fixture-company-scope",
      "fixture-session",
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Reprendre la dernière réponse" }),
    );
    expect((body as HTMLTextAreaElement).value).toBe("[FICTIF] Texte proposé");
    fireEvent.change(
      screen.getByRole("textbox", { name: "Titre du livrable" }),
      { target: { value: "[FICTIF] Brouillon" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Créer le brouillon" }));
    await screen.findByText("Le brouillon n’a pas pu être confirmé");
    fireEvent.click(screen.getByRole("button", { name: "Créer le brouillon" }));
    await waitFor(() => expect(artifactsApi.create).toHaveBeenCalledTimes(2));
    const calls = vi.mocked(artifactsApi.create).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][0]).toBe("fixture-company-scope");
    expect(calls[0][1]).toMatchObject({
      body_markdown: "[FICTIF] Texte proposé",
      sources: [{ kind: "session", public_id: "fixture-session" }],
    });
  });
  it("does not offer creation to a reader even if the URL requests it", async () => {
    vi.mocked(companyApi.overview).mockResolvedValue({
      ...fixtureCompany,
      workspace: { ...fixtureCompany.workspace, role: "viewer" },
    });
    openPage("/projects/fixture-project/artifacts?create=1");
    await screen.findByRole("heading", { name: "Livrables" });
    expect(
      screen.queryByRole("button", { name: "Créer le brouillon" }),
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Nouveau livrable" }),
    ).toBeNull();
  });
  it("rejects an inaccessible project without falling back to company data", async () => {
    openPage("/artifacts?project=other-workspace-project&create=1");
    await screen.findByText("Projet inaccessible");
    expect(artifactsApi.list).not.toHaveBeenCalled();
    expect(api.snapshot).toHaveBeenCalledWith("other-workspace-project");
  });
});

it("keeps an archived project's library accessible without a creation form", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue({
    ...fixtureCompany,
    projects: [],
  });
  vi.mocked(api.snapshot).mockResolvedValue({
    ...fixtureSnapshot,
    project: { ...fixtureSnapshot.project, status: "archived" },
  });
  openPage("/projects/fixture-project/artifacts?create=1");
  await screen.findByRole("heading", { name: "Livrables" });
  await screen.findByText(/Projet archivé : ses livrables/);
  expect(
    screen.queryByRole("button", { name: "Créer le brouillon" }),
  ).toBeNull();
  await waitFor(() =>
    expect(artifactsApi.list).toHaveBeenCalledWith(
      "fixture-project",
      expect.any(Object),
    ),
  );
});
