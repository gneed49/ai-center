// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  act,
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
import { setRequestIdentity } from "@/api/request-context";
import {
  fixtureTicketCoverage,
  fixtureTicketPreview,
  fixtureTicketResult,
  fixtureTicketTarget,
  fixtureTicketTools,
  fixtureTicketVersion,
} from "@/test-fixtures/ticket-publications";
import { TicketPublications } from "./ticket-publications";
import { TypedDraftView } from "@/components/artifacts/typed-draft-view";
import {
  newTicketCommand,
  saveTicketCommand,
  ticketCommandKey,
} from "./ticket-command";
import type { ArtifactVersion } from "@/api/artifact-types";
vi.mock("@/api/artifacts", () => ({ artifactsApi: { destinations: vi.fn() } }));
vi.mock("@/api/work-tools", () => ({
  workToolsApi: {
    settings: vi.fn(),
    ticketCoverage: vi.fn(),
    ticketPreview: vi.fn(),
    publishTickets: vi.fn(),
    ticketReceipt: vi.fn(),
    publication: vi.fn(),
    publications: vi.fn(),
  },
}));
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  setRequestIdentity("fixture-actor", "fixture-company", "fixture-token");
  vi.mocked(artifactsApi.destinations).mockResolvedValue({
    items: [
      {
        artifact_type: "product_tickets",
        provider: "linear",
        target_id: fixtureTicketTarget,
        label: "[FICTIF] Équipe produit",
        origin: "company",
        revision: 1,
        project_id: null,
      },
    ],
  });
  vi.mocked(workToolsApi.settings).mockResolvedValue(fixtureTicketTools);
  vi.mocked(workToolsApi.ticketCoverage).mockResolvedValue(
    fixtureTicketCoverage,
  );
  vi.mocked(workToolsApi.ticketPreview).mockImplementation(async (_id, input) =>
    fixtureTicketPreview(input),
  );
  vi.mocked(workToolsApi.publishTickets).mockRejectedValue(
    new Error("Réponse perdue"),
  );
  vi.mocked(workToolsApi.ticketReceipt).mockResolvedValue({
    status: "processing",
    can_retry: false,
    result: null,
  });
  vi.mocked(workToolsApi.publication).mockImplementation(async (id) => ({
    publication:
      fixtureTicketResult.publications.find((job) => job.public_id === id) ??
      fixtureTicketResult.publications[0],
    observations: [],
  }));
  vi.mocked(workToolsApi.publications).mockResolvedValue({
    items: fixtureTicketResult.publications,
    limit: 10,
    offset: 0,
  });
});
afterEach(cleanup);
function open(
  version: ArtifactVersion = fixtureTicketVersion,
  ticket: string | null = null,
) {
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
      <MemoryRouter
        initialEntries={[
          `/artifacts/fixture-artifact${ticket === null ? "" : `?version=${version.public_id}&ticket=${ticket}`}`,
        ]}
      >
        <TypedDraftView version={version} ticket={ticket} />
        <TicketPublications
          artifactId="fixture-artifact"
          projectId="fixture-project"
          version={version}
          artifactType="product_tickets"
          isCurrent
          canEdit
          currentVersionId={version.public_id}
          currentVersionNumber={version.version}
        >
          <p>Documentaire</p>
        </TicketPublications>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
async function select(index: number) {
  fireEvent.click(
    await screen.findByRole("checkbox", {
      name: `Sélectionner Ticket ${index} — [FICTIF] Travail ${index}`,
    }),
  );
}
async function prepare(count: number) {
  fireEvent.click(
    screen.getByRole("button", { name: `Préparer les ${count} tickets` }),
  );
  return screen.findByRole("region", { name: "Confirmation des tickets" });
}
it("covers all 30 tickets and previews only the explicit subset before creating anything", async () => {
  open();
  await select(2);
  await select(30);
  expect(screen.getAllByRole("checkbox")).toHaveLength(30);
  expect(workToolsApi.publishTickets).not.toHaveBeenCalled();
  const review = await prepare(2);
  expect(workToolsApi.ticketPreview).toHaveBeenCalledWith(
    "fixture-artifact",
    expect.objectContaining({ ticket_indexes: [1, 29] }),
  );
  expect(
    within(review).getByText("Description 30", { exact: false }),
  ).toBeDefined();
  expect(workToolsApi.publishTickets).not.toHaveBeenCalled();
  fireEvent.click(
    within(review).getByRole("button", {
      name: "Confirmer la création de 2 tickets dans Linear",
    }),
  );
  await waitFor(() =>
    expect(workToolsApi.publishTickets).toHaveBeenCalledTimes(1),
  );
  expect(vi.mocked(workToolsApi.publishTickets).mock.calls[0][1]).toMatchObject(
    {
      ticket_indexes: [1, 29],
      preview_fingerprint: "b".repeat(64),
      confirm_additional_issues: false,
    },
  );
});

it.each(["29", null])(
  "keeps explicit ticket focus when its receipt arrives later (ticket=%s)",
  async (ticket) => {
    const command = newTicketCommand(
      {
        version_id: fixtureTicketVersion.public_id,
        connection_id: "fixture-tool",
        expected_provider: "linear",
        expected_target_id: fixtureTicketTarget,
        ticket_indexes: [29],
        preview_fingerprint: "b".repeat(64),
        prior_publications_fingerprint: "a".repeat(64),
        confirm_additional_issues: false,
      },
      2,
    );
    saveTicketCommand(
      ticketCommandKey("fixture-artifact", "fixture-project"),
      command,
    );
    let resolveReceipt!: (
      receipt: Awaited<ReturnType<typeof workToolsApi.ticketReceipt>>,
    ) => void;
    vi.mocked(workToolsApi.ticketReceipt).mockReturnValue(
      new Promise((resolve) => {
        resolveReceipt = resolve;
      }),
    );
    open(fixtureTicketVersion, ticket);
    const entry = screen.getByRole("article", {
      name: "Ticket 30 · version 2",
    });
    if (ticket !== null) expect(document.activeElement).toBe(entry);
    await waitFor(() => expect(workToolsApi.ticketReceipt).toHaveBeenCalled());
    await act(async () => {
      resolveReceipt({
        status: "completed",
        can_retry: false,
        result: fixtureTicketResult,
      });
    });
    await screen.findByText(/3 nouvelle\(s\) demande\(s\) enregistrée\(s\)/);
    const expectedFocus =
      ticket === null
        ? screen.getByRole("region", { name: "Suivi des tickets demandés" })
        : entry;
    expect(document.activeElement).toBe(expectedFocus);
    expect(workToolsApi.publishTickets).not.toHaveBeenCalled();
  },
);
it("does not let incomplete coverage or a stale selection confirm an unseen batch", async () => {
  const first = open();
  await select(1);
  await prepare(1);
  await select(2);
  expect(
    screen.queryByRole("region", { name: "Confirmation des tickets" }),
  ).toBeNull();
  expect(workToolsApi.publishTickets).not.toHaveBeenCalled();
  first.unmount();
  vi.mocked(workToolsApi.ticketCoverage).mockResolvedValue({
    ...fixtureTicketCoverage,
    items: fixtureTicketCoverage.items.slice(0, 10),
  });
  open();
  await screen.findByRole("alert");
  expect(screen.queryByRole("button", { name: /Préparer les/ })).toBeNull();
});
it("requires explicit acknowledgement of earlier versions and respects the known quota", async () => {
  vi.mocked(workToolsApi.ticketPreview).mockImplementation(
    async (_id, input) => ({
      ...fixtureTicketPreview(input),
      prior_publications: fixtureTicketResult.publications,
      requires_additional_confirmation: true,
    }),
  );
  open();
  await select(1);
  const review = await prepare(1);
  const confirm = within(review).getByRole("button", {
    name: "Confirmer la création de 1 tickets dans Linear",
  }) as HTMLButtonElement;
  expect(confirm.disabled).toBe(true);
  fireEvent.click(
    within(review).getByRole("checkbox", { name: /Je confirme la création/ }),
  );
  expect(confirm.disabled).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Tout sélectionner" }));
  await prepare(30);
  expect(
    (
      screen.getByRole("button", {
        name: "Confirmer la création de 30 tickets dans Linear",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(screen.getByRole("alert").textContent).toContain(
    "capacité est insuffisante",
  );
});
it("recovers the original command after a lost response and a change of source version without another POST", async () => {
  const first = open();
  await select(1);
  const review = await prepare(1);
  fireEvent.click(
    within(review).getByRole("button", {
      name: "Confirmer la création de 1 tickets dans Linear",
    }),
  );
  await screen.findByText("La demande n’a pas pu être confirmée");
  const key = vi.mocked(workToolsApi.publishTickets).mock.calls[0][2];
  first.unmount();
  vi.mocked(workToolsApi.ticketReceipt).mockResolvedValue({
    status: "completed",
    can_retry: false,
    result: fixtureTicketResult,
  });
  const second = open({
    ...fixtureTicketVersion,
    public_id: "fixture-version-3",
    version: 3,
  });
  await screen.findByText(/3 nouvelle\(s\) demande\(s\) enregistrée\(s\)/);
  expect(
    screen.getByText(/Cette demande concerne une autre version/),
  ).toBeDefined();
  expect(workToolsApi.ticketReceipt).toHaveBeenCalledWith(
    "fixture-artifact",
    key,
  );
  expect(workToolsApi.publishTickets).toHaveBeenCalledTimes(1);
  expect(screen.getByText("Publication confirmée")).toBeDefined();
  expect(screen.getByText("Résultat à vérifier")).toBeDefined();
  expect(screen.getByText("Publication non aboutie")).toBeDefined();
  second.unmount();
  setRequestIdentity("another-actor", "fixture-company", "other-token");
  open();
  await screen.findByRole("region", { name: "Publication des tickets" });
  expect(
    screen.queryByRole("region", { name: "Suivi des tickets demandés" }),
  ).toBeNull();
  expect(workToolsApi.publishTickets).toHaveBeenCalledTimes(1);
});

it("uses the server receipt rather than an old local clock to resume a command never received", async () => {
  const command = newTicketCommand(
    {
      version_id: fixtureTicketVersion.public_id,
      connection_id: "fixture-tool",
      expected_provider: "linear",
      expected_target_id: fixtureTicketTarget,
      ticket_indexes: [0],
      preview_fingerprint: "b".repeat(64),
      prior_publications_fingerprint: "a".repeat(64),
      confirm_additional_issues: false,
    },
    2,
  );
  command.createdAt = 1; // An old local timestamp, independent of server retention.
  saveTicketCommand(
    ticketCommandKey("fixture-artifact", "fixture-project"),
    command,
  );
  vi.mocked(workToolsApi.ticketReceipt).mockResolvedValue({
    status: "not_received",
    can_retry: true,
    result: null,
  });
  open();
  const retry = await screen.findByRole("button", {
    name: "Reprendre la même demande",
  });
  expect(workToolsApi.publishTickets).not.toHaveBeenCalled();
  fireEvent.click(retry);
  await waitFor(() =>
    expect(workToolsApi.publishTickets).toHaveBeenCalledWith(
      "fixture-artifact",
      command.input,
      command.key,
    ),
  );
  expect(workToolsApi.publishTickets).toHaveBeenCalledTimes(1);
});
