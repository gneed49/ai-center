import { afterEach, describe, expect, it, vi } from "vitest";

import { api } from "./client";
import {
  RequestContextChangedError,
  setRequestIdentity,
} from "./request-context";

afterEach(() => vi.unstubAllGlobals());

describe("request identity boundary", () => {
  it("rejects an old body that finishes decoding after the actor changes", async () => {
    setRequestIdentity("actor-a", "workspace-a", "token-a");
    let finish!: (chunk: string) => void;
    const stream = new ReadableStream({
      start(controller) {
        finish = (chunk) => {
          controller.enqueue(new TextEncoder().encode(chunk));
          controller.close();
        };
      },
    });
    const fetch = vi.fn().mockResolvedValue(new Response(stream));
    vi.stubGlobal("fetch", fetch);
    const pending = api.projects();
    await Promise.resolve();
    const rejected = expect(pending).rejects.toBeInstanceOf(
      RequestContextChangedError,
    );
    setRequestIdentity("actor-b", null, "token-b");
    finish('[{"name":"private actor-a content"}]');
    await rejected;
    expect(fetch.mock.calls[0][1].signal.aborted).toBe(true);
    expect(fetch.mock.calls[0][1].headers).toMatchObject({
      Authorization: "Bearer token-a",
      "X-AI-Center-Workspace-Id": "workspace-a",
    });
  });

  it("keeps a pending response on token refresh but scopes the next request", async () => {
    setRequestIdentity("actor-refresh", "workspace-a", "old-token");
    const fetch = vi
      .fn()
      .mockImplementation(() => Promise.resolve(Response.json([])));
    vi.stubGlobal("fetch", fetch);
    const pending = api.projects();
    expect(
      setRequestIdentity("actor-refresh", "workspace-a", "fresh-token"),
    ).toBe(false);
    await expect(pending).resolves.toEqual([]);
    expect(fetch.mock.calls[0][1].signal.aborted).toBe(false);
    await api.projects();
    expect(fetch.mock.calls[1][1].headers.Authorization).toBe(
      "Bearer fresh-token",
    );
    setRequestIdentity("actor-refresh", "workspace-b", "fresh-token");
    expect(fetch.mock.calls[1][1].signal.aborted).toBe(true);
  });
});
