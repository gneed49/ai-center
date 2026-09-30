import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

// Fictional fixtures: these screenshots document UI rendering, never production data.
test("vérifie les écrans de l’entreprise avec des données synthétiques identifiées", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { companySuite: true });
  const pages = [
    ["overview", "/"],
    ["company", "/company"],
    ["agents", `/agents?project=${ids.project}`],
    ["artifact", "/artifacts/fixture-artifact"],
    ["tools", "/settings/tools"],
    ["code", `/projects/${ids.project}/code`],
  ];
  for (const [name, path] of pages) {
    await page.goto(path);
    await expect(
      page.getByRole("main").getByRole("heading", { level: 1 }),
    ).toBeVisible();
    await expect(
      page.getByRole("main").getByText(/Chargement/, { exact: false }),
    ).toHaveCount(0);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: testInfo.outputPath(`synthetic-${name}.png`),
      fullPage: true,
    });
    const axe = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(axe.violations).toEqual([]);
  }
  expect(errors).toEqual([]);
});
