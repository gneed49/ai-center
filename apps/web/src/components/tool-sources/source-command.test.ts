// @vitest-environment jsdom
import { beforeEach, expect, it } from "vitest";
import { setRequestIdentity } from "@/api/request-context";
import {
  newSourceCommand,
  normalizeSourceInput,
  readSourceCommand,
  saveSourceCommand,
  sourceCommandKey,
} from "./source-command";

const project = "20000000-0000-4000-8000-000000000001";
const connection = "60000000-0000-4000-8000-000000000001";
beforeEach(() => {
  localStorage.clear();
  setRequestIdentity("fixture-actor", "fixture-company", "fixture-token");
});

it("normalizes official locators while refusing secrets, redirects and ambiguous identities", () => {
  expect(normalizeSourceInput("linear", " prod-42 ")).toBe("PROD-42");
  expect(
    normalizeSourceInput(
      "linear",
      "https://linear.app/fictif/issue/PROD-42/title",
    ),
  ).toBe("https://linear.app/fictif/issue/PROD-42");
  expect(
    normalizeSourceInput(
      "notion",
      "https://www.notion.so/Fictif-70000000000040008000000000000001?pvs=4",
    ),
  ).toBe("https://www.notion.so/70000000000040008000000000000001");
  for (const value of [
    "https://linear.app.evil.invalid/fictif/issue/PROD-42",
    "https://fixture-secret@linear.app/fictif/issue/PROD-42",
    "https://linear.app:443/fictif/issue/PROD-42",
    "https://linear.app/fictif/issue/PROD-42?token=fixture",
    "https://linear.app/fictif/../fictif/issue/PROD-42",
    "https://linear.app/fictif/issue/PROD-42#fixture",
    "PROD-42\u0000",
  ])
    expect(() => normalizeSourceInput("linear", value)).toThrow();
  expect(() =>
    normalizeSourceInput(
      "notion",
      "https://notion.so/80000000000040008000000000000001/Fictif-70000000000040008000000000000001",
    ),
  ).toThrow();
});

it("keeps the exact command across reloads without trusting the device clock or another company", () => {
  const key = sourceCommandKey(project, "attach", null);
  const command = newSourceCommand(project, {
    action: "attach",
    referenceId: null,
    input: {
      connection_id: connection,
      expected_connection_revision: 1,
      provider: "linear",
      source: "PROD-42",
      confirm_scope_sharing: true,
    },
  });
  command.createdAt = 1; // Only the server may expire the durable receipt.
  saveSourceCommand(key, command);
  expect(readSourceCommand(key, project, "attach", null)).toEqual(command);
  setRequestIdentity("fixture-actor", "other-company", "other-fixture-token");
  const otherKey = sourceCommandKey(project, "attach", null);
  expect(otherKey).not.toBe(key);
  expect(readSourceCommand(otherKey, project, "attach", null)).toBeNull();
  localStorage.setItem(
    key,
    JSON.stringify({
      ...command,
      input: { ...command.input, api_key: "synthetic-not-a-secret" },
    }),
  );
  expect(readSourceCommand(key, project, "attach", null)).toBeNull();
});
