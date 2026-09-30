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
import { toolSourcesApi } from "@/api/tool-sources";
import { setRequestIdentity } from "@/api/request-context";
import { fixtureSourceResult } from "@/test-fixtures/tool-sources";
import { useSourceCommand } from "./use-source-command";
import {
  newSourceCommand,
  saveSourceCommand,
  sourceCommandKey,
} from "./source-command";

vi.mock("@/api/tool-sources", () => ({
  toolSourcesApi: { attach: vi.fn(), receipt: vi.fn() },
}));
const project = fixtureSourceResult.reference.project_id;
const input = {
  action: "attach" as const,
  referenceId: null,
  input: {
    connection_id: fixtureSourceResult.reference.connection_id,
    expected_connection_revision: 1,
    provider: "linear" as const,
    source: "PROD-42",
    confirm_scope_sharing: true as const,
  },
};
const complete = {
  status: "completed" as const,
  can_retry: false,
  retry_after: null,
  result: fixtureSourceResult,
  error: null,
};
const missing = {
  status: "not_received" as const,
  can_retry: true,
  retry_after: null,
  result: null,
  error: null,
};
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  setRequestIdentity("fixture-actor", "fixture-company", "fixture-token");
});
afterEach(cleanup);
function Harness() {
  const command = useSourceCommand(project, "attach", null);
  return (
    <>
      <button onClick={() => command.submit(input)}>Ajouter</button>
      <button onClick={command.retry}>Reprendre</button>
      <output>{command.receipt.data?.status ?? "nouveau"}</output>
    </>
  );
}
function mount(
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } }),
) {
  return {
    client,
    ...render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>,
    ),
  };
}
it("recovers a lost response by receipt and refreshes the source without another remote command", async () => {
  let accepted = false;
  vi.mocked(toolSourcesApi.attach).mockImplementation(async () => {
    accepted = true;
    throw new Error("[FICTIF] response lost");
  });
  vi.mocked(toolSourcesApi.receipt).mockImplementation(async () =>
    accepted ? complete : missing,
  );
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidated = vi.spyOn(client, "invalidateQueries");
  const view = mount(client);
  fireEvent.click(screen.getByRole("button", { name: "Ajouter" }));
  await waitFor(() =>
    expect(screen.getByRole("status").textContent).toBe("completed"),
  );
  await waitFor(() =>
    expect(invalidated).toHaveBeenCalledWith({
      queryKey: ["tool-source", fixtureSourceResult.reference.public_id],
    }),
  );
  expect(toolSourcesApi.attach).toHaveBeenCalledTimes(1);
  view.unmount();
  mount();
  await waitFor(() =>
    expect(screen.getByRole("status").textContent).toBe("completed"),
  );
  expect(toolSourcesApi.attach).toHaveBeenCalledTimes(1);
});
it("only repeats an interrupted command on explicit action, preserving its exact key and input", async () => {
  const command = newSourceCommand(project, input);
  saveSourceCommand(sourceCommandKey(project, "attach", null), command);
  vi.mocked(toolSourcesApi.receipt).mockResolvedValue({
    ...missing,
    status: "interrupted",
  });
  vi.mocked(toolSourcesApi.attach).mockResolvedValue(fixtureSourceResult);
  mount();
  await waitFor(() =>
    expect(screen.getByRole("status").textContent).toBe("interrupted"),
  );
  expect(toolSourcesApi.attach).not.toHaveBeenCalled();
  vi.mocked(toolSourcesApi.receipt).mockResolvedValue(complete);
  fireEvent.click(screen.getByRole("button", { name: "Reprendre" }));
  await waitFor(() =>
    expect(toolSourcesApi.attach).toHaveBeenCalledWith(
      project,
      command.input,
      command.key,
    ),
  );
  expect(toolSourcesApi.attach).toHaveBeenCalledTimes(1);
});
