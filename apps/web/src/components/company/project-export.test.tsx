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
import { companyApi } from "@/api/company";
import { companyControlsApi } from "@/api/company-controls";
import { ApiError } from "@/api/client";
import { setRequestIdentity } from "@/api/request-context";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { ProjectExport } from "./project-export";
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/company-controls", () => ({
  companyControlsApi: { exportProject: vi.fn() },
}));
let client: QueryClient;
let click: ReturnType<typeof vi.spyOn>;
const createObjectURL = vi.fn(),
  revokeObjectURL = vi.fn();
const exact =
  '{"format":"ai-center-project-data-v1", "complete":true,"data":{},"referenced_sources":[{"version":"old-exact"}],"excluded":["secrets"]}';
beforeEach(() => {
  vi.resetAllMocks();
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(companyControlsApi.exportProject).mockResolvedValue(exact);
  vi.stubGlobal(
    "URL",
    class extends URL {
      static createObjectURL = createObjectURL;
      static revokeObjectURL = revokeObjectURL;
    },
  );
  createObjectURL.mockReturnValue("blob:fixture-export");
  click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  client.clear();
  click.mockRestore();
  vi.unstubAllGlobals();
});
function open() {
  render(
    <QueryClientProvider client={client}>
      <ProjectExport projectId="fixture-archived-project" />
    </QueryClientProvider>,
  );
}
it("downloads canonical bytes including cited sources for a project absent from the active list", async () => {
  open();
  fireEvent.click(
    await screen.findByRole("button", { name: "Exporter ce projet" }),
  );
  await screen.findByText(/Téléchargement de l’export du projet demandé/);
  expect(companyControlsApi.exportProject).toHaveBeenCalledWith(
    "fixture-archived-project",
  );
  const blob = createObjectURL.mock.calls[0][0] as Blob;
  const reader = new FileReader();
  const text = new Promise((resolve) => {
    reader.onload = () => resolve(reader.result);
  });
  reader.readAsText(blob);
  expect(await text).toBe(exact);
  expect(click).toHaveBeenCalledOnce();
  expect((click.mock.instances[0] as HTMLAnchorElement).download).toBe(
    "ai-center-project-fixture-archived-project.json",
  );
  await waitFor(() =>
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:fixture-export"),
  );
});
it.each(["viewer", "editor"] as const)(
  "does not offer a company owner's export to %s",
  async (role) => {
    vi.mocked(companyApi.overview).mockResolvedValue({
      ...fixtureCompany,
      workspace: { ...fixtureCompany.workspace, role },
    });
    open();
    await waitFor(() => expect(companyApi.overview).toHaveBeenCalled());
    expect(
      screen.queryByRole("button", { name: "Exporter ce projet" }),
    ).toBeNull();
    expect(companyControlsApi.exportProject).not.toHaveBeenCalled();
  },
);
it("does not download a response returned after workspace switch", async () => {
  let finish!: (text: string) => void;
  vi.mocked(companyControlsApi.exportProject).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  open();
  fireEvent.click(
    await screen.findByRole("button", { name: "Exporter ce projet" }),
  );
  await waitFor(() => expect(finish).toBeDefined());
  setRequestIdentity(
    "fixture-actor",
    "fixture-other-workspace",
    "fixture-token",
  );
  finish(exact);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Exporter ce projet",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  expect(click).not.toHaveBeenCalled();
  expect(createObjectURL).not.toHaveBeenCalled();
});
it("does not turn an export limit error into a partial file", async () => {
  vi.mocked(companyControlsApi.exportProject).mockRejectedValue(
    new ApiError("L’export dépasse la limite autorisée.", 400),
  );
  open();
  fireEvent.click(
    await screen.findByRole("button", { name: "Exporter ce projet" }),
  );
  await screen.findByText("L’export du projet n’a pas abouti");
  expect(click).not.toHaveBeenCalled();
  expect(createObjectURL).not.toHaveBeenCalled();
});
