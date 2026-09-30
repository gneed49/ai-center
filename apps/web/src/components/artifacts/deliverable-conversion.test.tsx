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
import { MemoryRouter } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { companyApi } from "@/api/company";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { DeliverableConversion } from "./deliverable-conversion";
vi.mock("@/api/artifacts", () => ({
  artifactsApi: { fromDeliverable: vi.fn(), conversionReceipt: vi.fn() },
}));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
beforeEach(() => {
  localStorage.clear();
  vi.mocked(artifactsApi.conversionReceipt).mockResolvedValue({
    status: "interrupted",
    can_retry: true,
    result: null,
  });
});
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
it("converts the selected historical identity only after an explicit action and keeps retry identity", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(artifactsApi.fromDeliverable).mockRejectedValue(
    new Error("Interrompu"),
  );
  render(
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
        <DeliverableConversion
          projectId="project"
          deliverableId="historical-v1"
          version={1}
          archived={false}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  );
  const button = await screen.findByRole("button", {
    name: "Créer un brouillon depuis cette version",
  });
  expect(artifactsApi.fromDeliverable).not.toHaveBeenCalled();
  fireEvent.click(button);
  await screen.findByText("La conversion n’a pas pu être confirmée");
  await waitFor(() =>
    expect((button as HTMLButtonElement).disabled).toBe(false),
  );
  fireEvent.click(button);
  await waitFor(() =>
    expect(artifactsApi.fromDeliverable).toHaveBeenCalledTimes(2),
  );
  const calls = vi.mocked(artifactsApi.fromDeliverable).mock.calls;
  expect(calls[0]).toEqual(calls[1]);
  expect(calls[0].slice(0, 2)).toEqual(["project", "historical-v1"]);
});

it("restores a conversion after refresh without creating another draft", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(artifactsApi.fromDeliverable).mockRejectedValue(
    new Error("Réponse perdue"),
  );
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
          <DeliverableConversion
            projectId="project"
            deliverableId="historical-v1"
            version={1}
            archived={false}
          />
        </MemoryRouter>
      </QueryClientProvider>,
    );
  }
  const first = open();
  fireEvent.click(
    await screen.findByRole("button", {
      name: "Créer un brouillon depuis cette version",
    }),
  );
  await screen.findByText("La conversion n’a pas pu être confirmée");
  const key = vi.mocked(artifactsApi.fromDeliverable).mock.calls[0][2];
  first.unmount();
  vi.mocked(artifactsApi.conversionReceipt).mockResolvedValue({
    status: "processing",
    can_retry: false,
    result: null,
  });
  open();
  await screen.findByText(/Cette demande reste disponible après rechargement/);
  expect(artifactsApi.conversionReceipt).toHaveBeenCalledWith("project", key);
  expect(artifactsApi.fromDeliverable).toHaveBeenCalledTimes(1);
});
