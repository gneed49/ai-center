// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { api } from "./client";
import { setRequestIdentity } from "./request-context";
import { fixtureSession } from "@/test-fixtures/context-loop";
afterEach(() => vi.unstubAllGlobals());
it("preserves the original client id independently from the command key and raw content", async () => {
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
  const fetch = vi
    .fn()
    .mockResolvedValue(
      new Response(JSON.stringify(fixtureSession), { status: 200 }),
    );
  vi.stubGlobal("fetch", fetch);
  await api.sendMessage(
    "fixture-project",
    "fixture-session",
    "  [FICTIF] contenu brut  ",
    "fixture-key",
    "fixture-client-message",
  );
  const init = fetch.mock.calls[0][1];
  expect(init.headers["Idempotency-Key"]).toBe("fixture-key");
  expect(JSON.parse(init.body)).toEqual({
    content: "  [FICTIF] contenu brut  ",
    client_message_id: "fixture-client-message",
  });
});
