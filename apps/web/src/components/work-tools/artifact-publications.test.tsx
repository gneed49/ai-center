// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { workToolsApi } from "@/api/work-tools";
import { ApiError } from "@/api/client";
import {
  fixtureArtifactVersion,
  fixtureDestinations,
} from "@/test-fixtures/artifacts";
import { fixturePublication, fixtureTools } from "@/test-fixtures/work-tools";
import { ArtifactPublications } from "./artifact-publications";
import { PublicationDetailPanel } from "./publication-detail";
vi.mock("@/api/artifacts", () => ({ artifactsApi: { destinations: vi.fn() } }));
vi.mock("@/api/work-tools", () => ({
  workToolsApi: {
    settings: vi.fn(),
    publications: vi.fn(),
    publish: vi.fn(),
    publication: vi.fn(),
    cancel: vi.fn(),
    reconcile: vi.fn(),
    refresh: vi.fn(),
  },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(artifactsApi.destinations).mockResolvedValue(fixtureDestinations);
  vi.mocked(workToolsApi.settings).mockResolvedValue(fixtureTools);
  vi.mocked(workToolsApi.publications).mockResolvedValue({
    items: [],
    limit: 10,
    offset: 0,
  });
  vi.mocked(workToolsApi.publication).mockResolvedValue({
    publication: fixturePublication,
    observations: [],
  });
  vi.mocked(workToolsApi.publish).mockRejectedValue(
    new ApiError("Réponse interrompue", 504),
  );
  vi.mocked(workToolsApi.cancel).mockResolvedValue({
    ...fixturePublication,
    status: "cancelled",
  });
  vi.mocked(workToolsApi.reconcile).mockResolvedValue({
    ...fixturePublication,
    status: "succeeded",
  });
});
afterEach(() => {
  cleanup();
  client.clear();
});
function openPage(
  options: {
    status?: "draft" | "validated";
    current?: boolean;
    canEdit?: boolean;
  } = {},
) {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <ArtifactPublications
          artifactId="fixture-artifact"
          version={{
            ...fixtureArtifactVersion,
            status: options.status ?? "validated",
          }}
          isCurrent={options.current ?? true}
          artifactType="specification"
          projectId="fixture-project"
          canEdit={options.canEdit ?? true}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("explicit immutable-version publication", () => {
  it("waits for confirmation, preserves the displayed destination and retries the identical command after ambiguity", async () => {
    openPage();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Préparer la publication de la version 2",
      }),
    );
    expect(workToolsApi.publish).not.toHaveBeenCalled();
    await act(async () => {
      client.setQueryData(["artifact-destinations", "fixture-project"], {
        items: [
          {
            ...fixtureDestinations.items[0],
            target_id: "different-target",
            label: "[FICTIF] Nouvelle destination",
          },
        ],
      });
    });
    const review = screen.getByRole("region", {
      name: "Confirmation de publication",
    });
    expect(
      within(review).getByText(/Notion · \[FICTIF\] Documentation/),
    ).toBeDefined();
    fireEvent.click(
      within(review).getByRole("button", { name: "Confirmer la publication" }),
    );
    await screen.findByText(
      "La demande de publication n’a pas pu être confirmée",
    );
    fireEvent.click(
      within(review).getByRole("button", { name: "Confirmer la publication" }),
    );
    await waitFor(() => expect(workToolsApi.publish).toHaveBeenCalledTimes(2));
    const calls = vi.mocked(workToolsApi.publish).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][1]).toEqual({
      version_id: "fixture-artifact-version-2",
      connection_id: "fixture-tool",
      expected_provider: "notion",
      expected_target_id: "fixture-notion-page",
    });
  });
  it.each([
    { status: "draft" as const },
    { current: false },
    { canEdit: false },
  ])(
    "does not publish a draft, historical version or reader action: %j",
    async (options) => {
      openPage(options);
      await screen.findByText("Aucune publication enregistrée");
      expect(
        screen.queryByRole("button", { name: /Préparer la publication/ }),
      ).toBeNull();
      expect(workToolsApi.publish).not.toHaveBeenCalled();
    },
  );
  it("shows queued as pending and permits cancellation without claiming publication success", async () => {
    vi.mocked(workToolsApi.publications).mockResolvedValue({
      items: [fixturePublication],
      limit: 10,
      offset: 0,
    });
    openPage();
    fireEvent.click(
      await screen.findByRole("button", { name: "Détails et observations" }),
    );
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Annuler la demande en attente",
      }),
    );
    await waitFor(() =>
      expect(workToolsApi.cancel).toHaveBeenCalledWith(
        "fixture-publication",
        expect.any(String),
      ),
    );
    expect(screen.queryByText("Publication confirmée")).toBeNull();
    expect(workToolsApi.publish).not.toHaveBeenCalled();
  });
  it("reconciles an ambiguous result only with a valid object ID and never creates another object", async () => {
    vi.mocked(workToolsApi.publication).mockResolvedValue({
      publication: {
        ...fixturePublication,
        status: "needs_review",
        attempt_count: 1,
      },
      observations: [],
    });
    render(
      <QueryClientProvider client={client}>
        <PublicationDetailPanel publicationId="fixture-publication" canEdit />
      </QueryClientProvider>,
    );
    const input = await screen.findByRole("textbox", {
      name: "Identifiant de la page ou du ticket trouvé",
    });
    fireEvent.change(input, {
      target: { value: "https://untrusted.invalid/somewhere" },
    });
    expect(
      (
        screen.getByRole("button", {
          name: "Vérifier et rattacher cet objet",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    const id = "11111111-1111-4111-8111-111111111111";
    fireEvent.change(input, { target: { value: id } });
    fireEvent.click(
      screen.getByRole("button", { name: "Vérifier et rattacher cet objet" }),
    );
    await waitFor(() =>
      expect(workToolsApi.reconcile).toHaveBeenCalledWith(
        "fixture-publication",
        id,
        expect.any(String),
      ),
    );
    expect(workToolsApi.publish).not.toHaveBeenCalled();
    expect(workToolsApi.refresh).not.toHaveBeenCalled();
  });
});
