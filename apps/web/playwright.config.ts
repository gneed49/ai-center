import { defineConfig, devices } from "@playwright/test";

const port = process.env.AI_CENTER_WEB_E2E_PORT ?? "5173";
const authPort = process.env.AI_CENTER_WEB_AUTH_E2E_PORT ?? "5184";

export default defineConfig({
  testDir: "./e2e",
  outputDir: "/tmp/ai-center-playwright-results",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: "line",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: [
    {
      command: `npm run dev -w @ai-center/web -- --host 127.0.0.1 --port ${port} --strictPort`,
      url: `http://127.0.0.1:${port}`,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
    {
      command: `npm run dev -w @ai-center/web -- --host 127.0.0.1 --port ${authPort} --strictPort`,
      url: `http://127.0.0.1:${authPort}`,
      env: {
        VITE_SUPABASE_URL: "http://127.0.0.1:54329",
        VITE_SUPABASE_ANON_KEY: "public-local-fixture",
      },
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
  projects: [
    {
      name: "chromium-desktop",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
      },
    },
    {
      name: "chromium-compact",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1024, height: 768 },
      },
    },
    {
      name: "firefox-desktop",
      use: {
        ...devices["Desktop Firefox"],
        viewport: { width: 1440, height: 900 },
      },
    },
  ],
});
