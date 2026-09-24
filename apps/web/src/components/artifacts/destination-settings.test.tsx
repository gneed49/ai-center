// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { artifactsApi } from "@/api/artifacts";
import { ApiError } from "@/api/client";
import { fixtureDestinations } from "@/test-fixtures/artifacts";
import { DestinationSettings } from "./destination-settings";
vi.mock("@/api/artifacts", () => ({
  artifactsApi: {
    destinations: vi.fn(),
    setDestination: vi.fn(),
    resetDestination: vi.fn(),
  },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(artifactsApi.destinations).mockResolvedValue(fixtureDestinations);
  vi.mocked(artifactsApi.setDestination).mockRejectedValue(
    new ApiError("Délai dépassé", 504),
  );
  vi.mocked(artifactsApi.resetDestination).mockResolvedValue(
    fixtureDestinations,
  );
});
afterEach(() => {
  cleanup();
  client.clear();
});
function openSettings(
  projectId?: string,
  role: "owner" | "editor" | "viewer" = "owner",
) {
  render(
    <QueryClientProvider client={client}>
      <DestinationSettings projectId={projectId} role={role} />
    </QueryClientProvider>,
  );
}
describe("artifact destination inheritance", () => {
  it("creates a project override from company defaults with the project revision and a stable retry key", async () => {
    openSettings("fixture-project", "editor");
    await screen.findByText("Réglage de l’entreprise");
    expect(
      screen.getByText(/Ces réglages n’envoient aucun contenu/),
    ).toBeDefined();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Nom de la destination" }),
      { target: { value: "[FICTIF] Destination projet" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la destination" }),
    );
    await screen.findByText("Le réglage n’a pas pu être confirmé");
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la destination" }),
    );
    await waitFor(() =>
      expect(artifactsApi.setDestination).toHaveBeenCalledTimes(2),
    );
    const calls = vi.mocked(artifactsApi.setDestination).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][0]).toBe("fixture-project");
    expect(calls[0][1]).toMatchObject({
      expected_revision: 0,
      label: "[FICTIF] Destination projet",
      provider: "notion",
    });
    expect(artifactsApi.resetDestination).not.toHaveBeenCalled();
  });
  it("does not allow company settings edits by an editor", async () => {
    openSettings(undefined, "editor");
    await screen.findByText(
      "Seul le propriétaire peut modifier les destinations de l’entreprise.",
    );
    expect(
      screen.queryByRole("button", { name: "Enregistrer la destination" }),
    ).toBeNull();
    expect(
      (
        screen.getByRole("textbox", {
          name: "Nom de la destination",
        }) as HTMLInputElement
      ).disabled,
    ).toBe(true);
  });
  it("resets an explicit project setting using its exact revision", async () => {
    vi.mocked(artifactsApi.destinations).mockResolvedValue({
      items: [
        {
          ...fixtureDestinations.items[0],
          origin: "project",
          revision: 4,
          project_id: "fixture-project",
        },
      ],
    });
    openSettings("fixture-project");
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Revenir au réglage de l’entreprise",
      }),
    );
    await waitFor(() =>
      expect(artifactsApi.resetDestination).toHaveBeenCalledWith(
        "fixture-project",
        "specification",
        4,
        expect.any(String),
      ),
    );
  });
});
