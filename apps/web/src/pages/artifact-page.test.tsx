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
import { MemoryRouter, Route, Routes } from "react-router";
import { api, ApiError } from "@/api/client";
import { companyApi } from "@/api/company";
import { artifactsApi } from "@/api/artifacts";
import { setRequestIdentity } from "@/api/request-context";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { fixtureSnapshot } from "@/test-fixtures/context-loop";
import {
  fixtureArtifact,
  fixtureArtifactVersion,
  fixtureOldArtifactVersion,
} from "@/test-fixtures/artifacts";
import { ArtifactPage } from "./artifact-page";

vi.mock("@/components/work-tools/artifact-publications", () => ({
  ArtifactPublications: () => <p>Publication tested separately</p>,
}));
vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: { snapshot: vi.fn() },
}));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/artifacts", () => ({
  artifactsApi: {
    detail: vi.fn(),
    versions: vi.fn(),
    version: vi.fn(),
    save: vi.fn(),
    validate: vi.fn(),
    export: vi.fn(),
  },
}));
let client: QueryClient;
const writeText = vi.fn();
beforeEach(() => {
  vi.resetAllMocks();
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(artifactsApi.detail).mockResolvedValue(fixtureArtifact);
  vi.mocked(artifactsApi.versions).mockResolvedValue({
    items: [fixtureArtifactVersion, fixtureOldArtifactVersion],
    total: 2,
    limit: 25,
    offset: 0,
  });
  vi.mocked(artifactsApi.version).mockImplementation(async (_id, versionId) =>
    versionId === fixtureOldArtifactVersion.public_id
      ? fixtureOldArtifactVersion
      : fixtureArtifactVersion,
  );
  vi.mocked(artifactsApi.save).mockRejectedValue(
    new ApiError("Réponse interrompue", 504),
  );
  vi.mocked(artifactsApi.validate).mockRejectedValue(
    new ApiError("Réponse interrompue", 504),
  );
  vi.mocked(artifactsApi.export).mockResolvedValue(
    "[FICTIF] Export exact de v1",
  );
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
  writeText.mockResolvedValue(undefined);
});
afterEach(() => {
  cleanup();
  client.clear();
});
function openPage(search = "") {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/artifacts/fixture-artifact${search}`]}>
        <Routes>
          <Route path="/artifacts/:artifactId" element={<ArtifactPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("immutable artifact versions", () => {
  it("keeps the original base version and draft text after a concurrent update, retrying an ambiguous save with the same identity", async () => {
    openPage();
    fireEvent.click(
      await screen.findByRole("button", { name: "Nouvelle révision" }),
    );
    const body = await screen.findByRole("textbox", { name: "Contenu" });
    fireEvent.change(body, {
      target: { value: "[FICTIF] Ma révision en cours" },
    });
    await act(async () => {
      client.setQueryData(["artifact", "fixture-artifact"], {
        ...fixtureArtifact,
        current_version: {
          ...fixtureArtifactVersion,
          public_id: "fixture-concurrent-version",
          version: 3,
          body_markdown: "Autre édition",
        },
      });
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la nouvelle version" }),
    );
    await screen.findByText(
      "La révision n’a pas pu être confirmée ; votre saisie reste disponible",
    );
    expect((body as HTMLTextAreaElement).value).toBe(
      "[FICTIF] Ma révision en cours",
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la nouvelle version" }),
    );
    await waitFor(() => expect(artifactsApi.save).toHaveBeenCalledTimes(2));
    const calls = vi.mocked(artifactsApi.save).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][1].sources).toEqual([
      { kind: "knowledge", public_id: "fixture-knowledge-version" },
    ]);
    expect(calls[0][1]).toMatchObject({
      expected_version_id: "fixture-artifact-version-2",
      body_markdown: "[FICTIF] Ma révision en cours",
      sources: [{ kind: "knowledge", public_id: "fixture-knowledge-version" }],
      structured_content: fixtureArtifactVersion.structured_content,
    });
  });
  it("keeps validation retry bound to the exact version selected before a refresh", async () => {
    openPage();
    fireEvent.click(
      await screen.findByRole("button", { name: "Valider cette version" }),
    );
    await screen.findByText("La validation n’a pas pu être confirmée");
    await act(async () => {
      client.setQueryData(["artifact", "fixture-artifact"], {
        ...fixtureArtifact,
        current_version: {
          ...fixtureArtifactVersion,
          public_id: "fixture-concurrent-version",
          version: 3,
        },
      });
    });
    fireEvent.click(screen.getByRole("button", { name: "Réessayer" }));
    await waitFor(() => expect(artifactsApi.validate).toHaveBeenCalledTimes(2));
    const calls = vi.mocked(artifactsApi.validate).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][1]).toBe("fixture-artifact-version-2");
  });
  it("opens, compares and exports exact historical versions without offering edits", async () => {
    openPage(
      "?version=fixture-artifact-version-1&compare=fixture-artifact-version-2",
    );
    await screen.findByRole("heading", {
      name: "Comparer les versions 2 et 1",
    });
    expect(
      screen.queryByRole("button", { name: "Nouvelle révision" }),
    ).toBeNull();
    expect(screen.getByText("Une ancienne exigence.")).toBeDefined();
    expect(screen.getByText("Une exigence confirmée.")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Copier v1" }));
    await screen.findByText("Version 1 copiée avec ses sources.");
    expect(artifactsApi.export).toHaveBeenCalledWith(
      "fixture-artifact",
      "fixture-artifact-version-1",
      "markdown",
    );
    expect(writeText).toHaveBeenCalledWith("[FICTIF] Export exact de v1");
  });
});

it("keeps an archived project's exact versions visible without revision or validation controls", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue({
    ...fixtureCompany,
    projects: [],
  });
  openPage();
  await screen.findByRole("heading", {
    name: "[FICTIF] Spécification atelier",
  });
  expect(
    screen.queryByRole("button", { name: "Nouvelle révision" }),
  ).toBeNull();
  expect(
    screen.queryByRole("button", { name: "Valider cette version" }),
  ).toBeNull();
  expect(screen.getByLabelText("Version affichée")).toBeDefined();
});
