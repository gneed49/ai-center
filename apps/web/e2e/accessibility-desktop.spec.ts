import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("respecte les gates a11y, clavier et débordement desktop", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page);

  for (const path of [
    `/projects/${ids.project}`,
    `/projects/${ids.project}/sessions/${ids.techSession}`,
  ]) {
    await page.goto(path);
    await expect(page.locator("main")).toBeVisible();
    const audit = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    const blocking = audit.violations.filter((violation) =>
      ["critical", "serious"].includes(violation.impact ?? ""),
    );
    expect(
      blocking,
      JSON.stringify(
        blocking.map((violation) => ({
          id: violation.id,
          impact: violation.impact,
          targets: violation.nodes.flatMap((node) => node.target),
        })),
      ),
    ).toEqual([]);
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <=
          document.documentElement.clientWidth,
      ),
    ).toBe(true);
  }

  await page.keyboard.press("Tab");
  await expect
    .poll(() => page.evaluate(() => document.activeElement !== document.body))
    .toBe(true);
  expect(errors).toEqual([]);
});
