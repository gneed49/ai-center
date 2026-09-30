import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { fixtureArtifact } from "../src/test-fixtures/artifacts";
import { installMockApi, ids, captureConsoleErrors } from "./mock-api";
import { fixtureSession } from "../src/test-fixtures/context-loop";

test("conserve la consigne saisie immédiatement après un changement de type", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { companySuite: true });
  await page.route(
    `**/api/projects/${ids.project}/sessions/${ids.productSession}`,
    (route) =>
      route.fulfill({
        json: {
          ...fixtureSession,
          session: {
            ...fixtureSession.session,
            public_id: ids.productSession,
            project_id: ids.project,
          },
        },
      }),
  );
  await page.goto(
    `/projects/${ids.project}/artifacts?create=1&session=${ids.productSession}`,
  );
  const instructions = page.getByLabel("Ce que le document doit préparer");
  await expect(instructions).toBeVisible();
  for (let attempt = 0; attempt < 20; attempt++) {
    const type = attempt % 2 === 0 ? "product_tickets" : "specification";
    await page.getByLabel("Type de livrable").selectOption(type);
    await instructions.fill(`[FICTIF] Consigne immédiate ${attempt}`);
    await expect(instructions).toHaveValue(
      `[FICTIF] Consigne immédiate ${attempt}`,
    );
  }
  await page.screenshot({
    path: testInfo.outputPath("generation-input-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: testInfo.outputPath("generation-input-mobile.png"),
    fullPage: true,
  });
  const calls: unknown[] = [];
  await page.route(
    `**/api/projects/${ids.project}/artifact-generations/*`,
    (route) =>
      route.fulfill({
        json: {
          status: "completed",
          can_retry: false,
          result: fixtureArtifact,
        },
      }),
  );
  await page.route(
    `**/api/projects/${ids.project}/artifacts/generate`,
    async (route) => {
      calls.push(route.request().postDataJSON());
      await route.fulfill({ json: fixtureArtifact });
    },
  );
  await page
    .getByRole("button", { name: "Générer le brouillon avec l’agent" })
    .click();
  await expect(page).toHaveURL(/\/artifacts\/fixture-artifact$/);
  expect(calls).toEqual([
    {
      artifact_type: "specification",
      session_id: ids.productSession,
      instructions: "[FICTIF] Consigne immédiate 19",
    },
  ]);
  expect(errors).toEqual([]);
});
