import { describe, expect, it } from "vitest";

import { isAndroid, resolveApiUrl } from "./url";

describe("API URL resolution", () => {
  it("uses the Android emulator host inside an Android WebView", () => {
    expect(
      resolveApiUrl({
        userAgent:
          "Mozilla/5.0 (Linux; Android 15; sdk_gphone64_x86_64) AppleWebKit/537.36",
      }),
    ).toBe("http://10.0.2.2:4317");
  });

  it("keeps localhost for web and desktop", () => {
    expect(
      resolveApiUrl({ userAgent: "Mozilla/5.0 (X11; Linux x86_64)" }),
    ).toBe("http://127.0.0.1:4317");
    expect(isAndroid("Tauri/Linux")).toBe(false);
  });

  it("prefers and normalizes the build-time URL", () => {
    expect(
      resolveApiUrl({
        configuredUrl: " https://api.ai-center.example/ ",
        userAgent: "Android",
      }),
    ).toBe("https://api.ai-center.example");
  });
});
