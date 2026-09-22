import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { ids, installMockApi, captureConsoleErrors } from "./mock-api";

test("télécharge un export canonique du projet choisi avec les exclusions", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { companySuite: true });
  await page.goto(`/projects/${ids.project}`);
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Exporter ce projet", exact: true })
    .click();
  const file = await download;
  expect(file.suggestedFilename()).toBe(
    `ai-center-project-${ids.project}.json`,
  );
  const parsed = JSON.parse(await readFile((await file.path())!, "utf8"));
  expect(parsed).toMatchObject({
    format: "ai-center-project-data-v1",
    project_public_id: ids.project,
    complete: true,
    referenced_sources: [],
    excluded: ["credentials"],
  });
  expect(
    state.requests.filter((request) =>
      request.url().endsWith(`/api/projects/${ids.project}/export`),
    ),
  ).toHaveLength(1);
  expect(errors).toEqual([]);
});
