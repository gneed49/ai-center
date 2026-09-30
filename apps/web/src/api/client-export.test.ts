import { afterEach, describe, expect, it, vi } from "vitest";
import { api, ApiError } from "./client";
import {
  RequestContextChangedError,
  setRequestIdentity,
} from "./request-context";

afterEach(() => vi.unstubAllGlobals());

describe("canonical context export HTTP boundary", () => {
  it("keeps Markdown intact and sends the current identity without caching", async () => {
    setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
    const content = "# ContextPack fixture v2\n\n- Source: `fixture-version`\n";
    const fetch = vi.fn().mockResolvedValue(new Response(content));
    vi.stubGlobal("fetch", fetch);
    expect(await api.exportContextPack("fixture-pack", "markdown")).toBe(
      content,
    );
    expect(fetch.mock.calls[0][0]).toContain(
      "/api/context-packs/fixture-pack/export?format=markdown",
    );
    expect(fetch.mock.calls[0][1]).toMatchObject({
      cache: "no-store",
      headers: {
        Authorization: "Bearer fixture-token",
        "X-AI-Center-Workspace-Id": "fixture-workspace",
      },
    });
  });

  it("preserves structured API failures instead of downloading an error document", async () => {
    setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          Response.json(
            { message: "Accès refusé", code: "forbidden" },
            { status: 403 },
          ),
        ),
    );
    await expect(
      api.exportContextPack("fixture-pack", "json"),
    ).rejects.toMatchObject({
      status: 403,
      code: "forbidden",
      message: "Accès refusé",
    } satisfies Partial<ApiError>);
  });

  it("rejects export bytes read after a workspace switch", async () => {
    setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
    let finish!: (value: string) => void;
    const response = new Response();
    vi.spyOn(response, "text").mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));
    const pending = api.exportContextPack("fixture-pack", "markdown");
    await vi.waitFor(() => expect(finish).toBeDefined());
    const rejected = expect(pending).rejects.toBeInstanceOf(
      RequestContextChangedError,
    );
    setRequestIdentity(
      "fixture-actor",
      "fixture-other-workspace",
      "fixture-token",
    );
    finish("private previous workspace content");
    await rejected;
  });
});
