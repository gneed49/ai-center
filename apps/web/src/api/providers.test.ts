import { afterEach, describe, expect, it, vi } from "vitest";
import { providersApi } from "./providers";
import {
  RequestContextChangedError,
  setRequestIdentity,
} from "./request-context";

afterEach(() => vi.unstubAllGlobals());

describe("provider HTTP boundary", () => {
  it("scopes secret mutations and keeps keys out of the URL", async () => {
    setRequestIdentity("actor-a", "workspace-a", "fixture-token");
    const fetch = vi
      .fn()
      .mockResolvedValue(Response.json({ id: "connection" }));
    vi.stubGlobal("fetch", fetch);
    const input = {
      id: "connection",
      provider: "kimi",
      name: "Travail",
      model: "model",
      api_key: "fixture-only-key",
    };
    await providersApi.create(input, "retry-identity");
    const [url, init] = fetch.mock.calls[0];
    expect(url).toContain("/api/ai/connections");
    expect(url).not.toContain(input.api_key);
    expect(init.headers).toMatchObject({
      Authorization: "Bearer fixture-token",
      "X-AI-Center-Workspace-Id": "workspace-a",
      "Idempotency-Key": "retry-identity",
    });
    expect(JSON.parse(init.body)).toEqual(input);
    expect(init.cache).toBe("no-store");
  });
  it("omits an unchanged key and sends test actions without a prompt", async () => {
    setRequestIdentity("actor-b", "workspace-a", "fixture-token");
    const fetch = vi
      .fn()
      .mockImplementation(() => Promise.resolve(Response.json({})));
    vi.stubGlobal("fetch", fetch);
    await providersApi.update(
      "connection",
      { name: "Renamed", model: "new-model" },
      "retry-identity",
    );
    expect(JSON.parse(fetch.mock.calls[0][1].body)).not.toHaveProperty(
      "api_key",
    );
    await providersApi.test("connection");
    expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({});
  });
  it("rejects late private settings when the workspace changes", async () => {
    setRequestIdentity("actor-a", "workspace-a", "fixture-token");
    let finish!: (response: Response) => void;
    vi.stubGlobal(
      "fetch",
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            finish = resolve;
          }),
      ),
    );
    const pending = providersApi.settings();
    const rejected = expect(pending).rejects.toBeInstanceOf(
      RequestContextChangedError,
    );
    setRequestIdentity("actor-a", "workspace-b", "fixture-token");
    finish(Response.json({ connections: [{ name: "private-workspace-a" }] }));
    await rejected;
  });
});
