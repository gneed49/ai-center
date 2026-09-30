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
import { MemoryRouter, Route, Routes } from "react-router";
import { api, ApiError } from "@/api/client";
import { fixtureSession, fixtureSnapshot } from "@/test-fixtures/context-loop";
import { SessionPage } from "./session-page";

vi.mock("@/api/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/api/client")>()),
  api: { session: vi.fn(), snapshot: vi.fn(), sendMessage: vi.fn() },
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn(), success: vi.fn() } }));
let client: QueryClient;

beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(api.session).mockResolvedValue(fixtureSession);
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(api.sendMessage).mockRejectedValue(
    new ApiError("Erreur HTTP 504", 504),
  );
});
afterEach(() => {
  cleanup();
  client.clear();
});

async function openSession(expectComposer = true) {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter
        initialEntries={["/projects/fixture-project/sessions/fixture-session"]}
      >
        <Routes>
          <Route
            path="projects/:projectId/sessions/:sessionId"
            element={<SessionPage />}
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
  return expectComposer
    ? screen.findByRole("textbox", { name: "Message à l’agent product" })
    : screen.findByText(/Projet archivé : cette conversation/);
}

describe("session message recovery after an ambiguous timeout", () => {
  it("reuses the same command from both Send and Retry and retains the draft", async () => {
    const input = await openSession();
    fireEvent.change(input, {
      target: { value: "[FICTIF] Ajouter une exigence" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Envoyer" }));
    await screen.findByText("La réponse n’a pas pu être confirmée");
    expect(screen.queryByText("Le message n’a pas été envoyé")).toBeNull();
    expect(
      screen.getByText(/Le message peut déjà être enregistré/),
    ).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Envoyer" }));
    await waitFor(() => expect(api.sendMessage).toHaveBeenCalledTimes(2));
    await screen.findByText("La réponse n’a pas pu être confirmée");
    fireEvent.click(screen.getByRole("button", { name: "Réessayer" }));
    await waitFor(() => expect(api.sendMessage).toHaveBeenCalledTimes(3));
    const calls = vi.mocked(api.sendMessage).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[2]).toEqual(calls[0]);
    expect((input as HTMLTextAreaElement).value).toBe(calls[0][2]);
  });

  it("preserves identity after temporary editing, but creates a command for changed content", async () => {
    const input = await openSession();
    fireEvent.change(input, { target: { value: "[FICTIF] Demande initiale" } });
    fireEvent.click(screen.getByRole("button", { name: "Envoyer" }));
    await screen.findByText("La réponse n’a pas pu être confirmée");
    fireEvent.change(input, { target: { value: "édition temporaire" } });
    fireEvent.change(input, {
      target: { value: "  [FICTIF] Demande initiale  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Envoyer" }));
    await waitFor(() => expect(api.sendMessage).toHaveBeenCalledTimes(2));
    await screen.findByText("La réponse n’a pas pu être confirmée");
    expect(vi.mocked(api.sendMessage).mock.calls[1]).toEqual(
      vi.mocked(api.sendMessage).mock.calls[0],
    );
    fireEvent.change(input, { target: { value: "[FICTIF] Nouvelle demande" } });
    fireEvent.click(screen.getByRole("button", { name: "Envoyer" }));
    await waitFor(() => expect(api.sendMessage).toHaveBeenCalledTimes(3));
    const calls = vi.mocked(api.sendMessage).mock.calls;
    expect(calls[2][2]).toBe("[FICTIF] Nouvelle demande");
    expect(calls[2][3]).not.toBe(calls[0][3]);
  });
});

const receipt = {
  message_public_id: "fixture-owned-message",
  client_message_id: "fixture-original-client-id",
  idempotency_key: "fixture-original-key",
  submitted_content: "  [FICTIF] Demande exacte avec espaces  ",
  status: "interrupted" as const,
  can_retry: true,
  locked_until: null,
  retry_after_seconds: null,
  error_code: null,
  error_message: null,
  updated_at: "2026-09-21T10:00:00Z",
};
it("recovers after reload only on explicit action with original exact content and both identities", async () => {
  vi.mocked(api.session).mockResolvedValue({
    ...fixtureSession,
    message_commands: [receipt],
  });
  const draft = await openSession();
  fireEvent.change(draft, {
    target: { value: "[FICTIF] Autre brouillon en cours" },
  });
  expect(api.sendMessage).not.toHaveBeenCalled();
  vi.mocked(api.sendMessage).mockResolvedValue({
    ...fixtureSession,
    message_commands: [{ ...receipt, status: "completed", can_retry: false }],
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Reprendre cette demande" }),
  );
  await waitFor(() =>
    expect(api.sendMessage).toHaveBeenCalledWith(
      "fixture-project",
      "fixture-session",
      receipt.submitted_content,
      receipt.idempotency_key,
      receipt.client_message_id,
    ),
  );
  expect((draft as HTMLTextAreaElement).value).toBe(
    "[FICTIF] Autre brouillon en cours",
  );
});
it("does not expose retry for processing or permanent receipts and never sends from a read", async () => {
  vi.mocked(api.session).mockResolvedValue({
    ...fixtureSession,
    message_commands: [
      { ...receipt, status: "processing", can_retry: false },
      {
        ...receipt,
        idempotency_key: "permanent-key",
        status: "failed",
        can_retry: false,
      },
    ],
  });
  await openSession();
  expect(screen.getByText("Réponse en préparation")).toBeDefined();
  expect(
    screen.queryByRole("button", { name: "Reprendre cette demande" }),
  ).toBeNull();
  expect(api.sendMessage).not.toHaveBeenCalled();
});
it("attributes own, colleague and legacy messages without inventing an author", async () => {
  const base = {
    role: "user" as const,
    content: "[FICTIF] Décision",
    agent_scope: null,
    metadata: {},
    created_at: "2026-09-21T10:00:00Z",
  };
  vi.mocked(api.session).mockResolvedValue({
    ...fixtureSession,
    messages: [
      {
        ...base,
        public_id: "own",
        is_own: true,
        author_name: "[FICTIF] Camille",
      },
      {
        ...base,
        public_id: "colleague",
        is_own: false,
        author_name: "[FICTIF] Lina",
      },
      { ...base, public_id: "legacy", is_own: false, author_name: null },
    ],
  });
  await openSession();
  expect(screen.getByText("Vous")).toBeDefined();
  expect(screen.getByText("[FICTIF] Lina")).toBeDefined();
  expect(screen.getByText("Membre non identifié")).toBeDefined();
});
it("keeps archived conversations readable with no send or recovery action", async () => {
  vi.mocked(api.snapshot).mockResolvedValue({
    ...fixtureSnapshot,
    project: { ...fixtureSnapshot.project, status: "archived" },
  });
  vi.mocked(api.session).mockResolvedValue({
    ...fixtureSession,
    message_commands: [receipt],
    messages: [
      {
        public_id: "legacy",
        role: "user",
        content: "[FICTIF] Historique conservé",
        agent_scope: null,
        metadata: {},
        created_at: "2026-09-21T10:00:00Z",
      },
    ],
  });
  await openSession(false);
  expect(screen.getByText("[FICTIF] Historique conservé")).toBeDefined();
  expect(screen.queryByRole("textbox")).toBeNull();
  expect(
    screen.queryByRole("button", { name: "Reprendre cette demande" }),
  ).toBeNull();
  expect(api.sendMessage).not.toHaveBeenCalled();
});
