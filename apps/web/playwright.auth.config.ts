import { defineConfig, devices } from "@playwright/test";
import { dirname, join } from "node:path";

const fixture = process.env.AI_CENTER_AUTH_BROWSER_FIXTURE;
if (!fixture) throw new Error("Private guarded Auth fixture required");
const privateDirectory = dirname(fixture);

export default defineConfig({
  testDir: "./auth-real-e2e",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 60_000,
  outputDir: join(privateDirectory, "artifacts"),
  reporter: [["json", { outputFile: join(privateDirectory, "report.json") }]],
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:5183",
    // Real ephemeral JWTs and invitation fragments must never enter diagnostics.
    trace: "off",
    screenshot: "off",
    video: "off",
  },
});
