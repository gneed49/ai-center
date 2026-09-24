import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { workToolsApi } from "./work-tools";
import { setRequestIdentity } from "./request-context";
import { fixturePublication } from "@/test-fixtures/work-tools";
import {
  fixtureTicketCoverage,
  fixtureTicketPreview,
} from "@/test-fixtures/ticket-publications";
beforeEach(() =>
  setRequestIdentity("fixture-actor", "fixture-company", "fixture-token"),
);
afterEach(() => vi.unstubAllGlobals());
it("hydrates legacy receipt display metadata by GET while retaining the recorded status and document index", async () => {
  const legacy = { ...fixturePublication, status: "queued" };
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(Response.json(legacy))
    .mockResolvedValueOnce(
      Response.json({
        publication: {
          ...legacy,
          status: "succeeded",
          source_ticket_index: -1,
          title: "[FICTIF] Ancien document",
          source_version_number: 2,
        },
        observations: [],
      }),
    );
  vi.stubGlobal("fetch", fetch);
  const result = await workToolsApi.publish(
    "fixture-artifact",
    {
      version_id: "v2",
      connection_id: "fixture-tool",
      expected_provider: "notion",
      expected_target_id: "page",
    },
    "same-command",
  );
  expect(result).toMatchObject({
    status: "queued",
    source_ticket_index: -1,
    title: "[FICTIF] Ancien document",
    source_version_number: 2,
  });
  expect(fetch).toHaveBeenCalledTimes(2);
  expect(fetch.mock.calls[1][0]).toContain(
    "/api/publications/fixture-publication",
  );
  expect(fetch.mock.calls[1][1].method).toBeUndefined();
});
it("reads full coverage and previews without reserving a command or supplying replacement content", async () => {
  const input = {
    version_id: "v2",
    connection_id: "tool",
    expected_provider: "linear" as const,
    expected_target_id: "team",
    ticket_indexes: [1, 29],
  };
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(Response.json(fixtureTicketCoverage))
    .mockResolvedValueOnce(Response.json(fixtureTicketPreview(input)));
  vi.stubGlobal("fetch", fetch);
  await workToolsApi.ticketCoverage("artifact", "v2", "linear", "team");
  await workToolsApi.ticketPreview("artifact", input);
  const url = new URL(fetch.mock.calls[0][0], "http://localhost");
  expect(Object.fromEntries(url.searchParams)).toEqual({
    version_id: "v2",
    provider: "linear",
    target_id: "team",
  });
  expect(fetch.mock.calls[1][1].body).toBe(JSON.stringify(input));
  expect(fetch.mock.calls[1][1].headers["Idempotency-Key"]).toBeUndefined();
});
