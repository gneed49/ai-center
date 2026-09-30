import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { toolSourcesApi } from "./tool-sources";
import { setRequestIdentity } from "./request-context";
import {
  fixtureSourceObservation,
  fixtureSourceReference,
} from "@/test-fixtures/tool-sources";

beforeEach(() =>
  setRequestIdentity("fixture-actor", "fixture-company", "fixture-token"),
);
afterEach(() => vi.unstubAllGlobals());
it("opens an exact immutable observation by GET and refuses a substituted citation", async () => {
  const data = fixtureSourceObservation;
  const fetch = vi.fn().mockResolvedValueOnce(Response.json(data));
  vi.stubGlobal("fetch", fetch);
  await expect(
    toolSourcesApi.observation(
      "tool_source_observation",
      data.observation.public_id,
    ),
  ).resolves.toEqual(data);
  expect(fetch.mock.calls[0][0]).toContain(
    `/api/tool-source-observations/${data.observation.public_id}`,
  );
  expect(fetch.mock.calls[0][1].method).toBeUndefined();
  for (const change of [
    { public_id: "40000000-0000-4000-8000-000000000099" },
    { source_kind: "publication_observation" },
    { mandatory: true },
    { trust: "confirmed" },
  ]) {
    fetch.mockResolvedValueOnce(
      Response.json({
        ...data,
        observation: { ...data.observation, ...change },
      }),
    );
    await expect(
      toolSourcesApi.observation(
        "tool_source_observation",
        data.observation.public_id,
      ),
    ).rejects.toThrow("lecture demandée");
  }
});
it("refuses sources returned from another project or reference", async () => {
  const detail = {
    reference: fixtureSourceReference,
    observation: fixtureSourceObservation.observation,
  };
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({
        items: [detail],
        total_count: 1,
        active_count: 1,
        limit: 25,
        next_cursor: null,
      }),
    );
  vi.stubGlobal("fetch", fetch);
  await expect(
    toolSourcesApi.list("20000000-0000-4000-8000-000000000099"),
  ).rejects.toThrow("contexte demandé");
  fetch.mockResolvedValueOnce(
    Response.json({
      items: [detail.observation],
      total_count: 1,
      limit: 25,
      next_cursor: null,
    }),
  );
  await expect(
    toolSourcesApi.history("30000000-0000-4000-8000-000000000099"),
  ).rejects.toThrow("autre source");
});
