import { defineConfig, devices } from "@playwright/test";

function loopback(name: string, alias: string) {
  const raw = process.env[name] ?? process.env[alias];
  if (!raw) throw new Error(`The guarded ticket harness must provide ${name}.`);
  const url = new URL(raw);
  if (
    url.protocol !== "http:" ||
    url.hostname !== "127.0.0.1" ||
    !url.port ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  )
    throw new Error(
      `${name} must be an exact loopback origin supplied by the guarded harness.`,
    );
  return url.origin;
}
const web = loopback("E2E_WEB_URL", "AI_CENTER_REAL_E2E_WEB_URL");
loopback("E2E_API_URL", "AI_CENTER_REAL_E2E_API_URL");
const workspace = process.env.E2E_WORKSPACE_ID ?? process.env.VITE_WORKSPACE_ID;
if (
  !workspace ||
  !/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(
    workspace,
  )
)
  throw new Error(
    "The guarded ticket harness must provide the fixture workspace UUID.",
  );
export default defineConfig({
  testDir: "./ticket-real-e2e",
  outputDir: "/tmp/ai-center-ticket-real-playwright-results",
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  timeout: 120_000,
  expect: { timeout: 15_000 },
  reporter: "line",
  use: {
    baseURL: web,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    extraHTTPHeaders: { "X-AI-Center-Workspace-Id": workspace },
  },
  projects: [
    {
      name: "chromium-ticket-real-api",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
      },
    },
  ],
});
