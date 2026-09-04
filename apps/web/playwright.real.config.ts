import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./real-e2e",
  outputDir: "/tmp/ai-center-real-playwright-results",
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: process.env.AI_CENTER_REAL_E2E_WEB_URL ?? "http://127.0.0.1:5183",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium-desktop-real-api",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
      },
    },
  ],
});
