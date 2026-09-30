import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { artifactsApi } from "./artifacts";
import {
  RequestContextChangedError,
  setRequestIdentity,
} from "./request-context";
import { fixtureArtifact } from "@/test-fixtures/artifacts";

beforeEach(() =>
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token"),
);
afterEach(() => vi.unstubAllGlobals());
describe("artifact API identity and immutable version boundary", () => {
  it("preserves exact source versions, concurrency expectations and command identity on save", async () => {
    const fetch = vi.fn().mockResolvedValue(Response.json(fixtureArtifact));
    vi.stubGlobal("fetch", fetch);
    const input = {
      title: "[FICTIF] Version",
      body_markdown: "Contenu",
      structured_content: {},
      sources: [
        { kind: "knowledge" as const, public_id: "fixture-knowledge-version" },
      ],
      expected_version_id: "fixture-base-version",
    };
    await artifactsApi.save("fixture-artifact", input, "fixture-command");
    expect(fetch.mock.calls[0][0]).toContain(
      "/api/artifacts/fixture-artifact/draft",
    );
    expect(fetch.mock.calls[0][1]).toMatchObject({
      method: "POST",
      body: JSON.stringify(input),
      headers: {
        "Idempotency-Key": "fixture-command",
        Authorization: "Bearer fixture-token",
        "X-AI-Center-Workspace-Id": "fixture-workspace",
      },
    });
  });
  it("sends validated filters and pagination without losing punctuation in search", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValue(
        Response.json({ items: [], total: 0, limit: 25, offset: 25 }),
      );
    vi.stubGlobal("fetch", fetch);
    await artifactsApi.list("fixture-project", {
      q: "règles & sources",
      type: "specification",
      status: "validated",
      limit: 25,
      offset: 25,
    });
    const url = new URL(fetch.mock.calls[0][0], "http://localhost");
    expect(Object.fromEntries(url.searchParams)).toEqual({
      q: "règles & sources",
      type: "specification",
      status: "validated",
      limit: "25",
      offset: "25",
    });
  });
  it("exports canonical bytes for the requested historical version without caching", async () => {
    const content = "# [FICTIF] v1\n\nSource: fixture-version\n";
    const fetch = vi.fn().mockResolvedValue(new Response(content));
    vi.stubGlobal("fetch", fetch);
    expect(
      await artifactsApi.export(
        "fixture-artifact",
        "fixture-version-1",
        "markdown",
      ),
    ).toBe(content);
    expect(fetch.mock.calls[0][0]).toContain(
      "/api/artifacts/fixture-artifact/versions/fixture-version-1/export?format=markdown",
    );
    expect(fetch.mock.calls[0][1]).toMatchObject({ cache: "no-store" });
  });
  it("rejects export bytes completing after an identity switch", async () => {
    let finish!: (value: string) => void;
    const response = new Response();
    vi.spyOn(response, "text").mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));
    const pending = artifactsApi.export(
      "fixture-artifact",
      "fixture-version-1",
      "json",
    );
    await vi.waitFor(() => expect(finish).toBeDefined());
    const rejected = expect(pending).rejects.toBeInstanceOf(
      RequestContextChangedError,
    );
    setRequestIdentity(
      "fixture-actor",
      "fixture-other-workspace",
      "fixture-token",
    );
    finish("[FICTIF] Previous workspace private data");
    await rejected;
  });
});
