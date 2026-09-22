import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("explore les relations sourcées du graphe au clavier et en liste", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page);
  await page.route("**/api/company/graph", (route) =>
    route.fulfill({
      json: {
        workspace_public_id: "01000000-0000-4000-8000-000000000001",
        project_public_id: null,
        source_graph_versions: [
          { project_public_id: ids.project, graph_version: 4 },
        ],
        truncated: false,
        limits: { max_nodes: 500, max_edges: 1000 },
        nodes: [
          {
            id: ids.project,
            kind: "scope",
            project_public_id: ids.project,
            scope_kind: "project",
            label: "[FICTIF] Portail partenaires",
            status: "active",
            version_public_id: null,
            source_url: null,
          },
          {
            id: ids.version,
            kind: "knowledge",
            project_public_id: ids.project,
            scope_kind: "project",
            label: "[FICTIF] Accès sur invitation",
            status: "confirmed",
            version_public_id: ids.version,
            source_url: null,
          },
          {
            id: ids.deliverable,
            kind: "artifact",
            project_public_id: ids.project,
            scope_kind: "project",
            label: "[FICTIF] Spécification du portail",
            status: "validated",
            version_public_id: ids.deliverable,
            source_url: "https://www.notion.so/example",
          },
        ],
        edges: [
          {
            id: "link-rule",
            source_kind: "artifact",
            source_public_id: ids.deliverable,
            source_project_public_id: ids.project,
            target_kind: "knowledge",
            target_public_id: ids.version,
            target_project_public_id: ids.project,
            edge_type: "derived_from",
            status: "confirmed",
            provenance: { origin: "source_record", persisted: true },
          },
        ],
      },
    }),
  );
  await page.goto("/graph");
  await expect(
    page.getByRole("heading", { name: "Le graphe de votre entreprise" }),
  ).toBeVisible();
  const artifact = page.getByRole("button", {
    name: "[FICTIF] Spécification du portail, Livrable, Projet",
  });
  await artifact.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Spécification du portail" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Ouvrir la source" }),
  ).toHaveAttribute("href", "https://www.notion.so/example");
  await page
    .getByRole("button", { name: "[FICTIF] Accès sur invitation", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Accès sur invitation" }),
  ).toBeVisible();
  await page.getByText("Version de la source", { exact: true }).click();
  await expect(page.getByText(ids.version, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Liste", exact: true }).click();
  await expect(
    page.getByRole("list", { name: "Éléments du graphe" }),
  ).toBeVisible();
  const audit = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.filter((item) =>
      ["critical", "serious"].includes(item.impact ?? ""),
    ),
  ).toEqual([]);
  await page.screenshot({
    path: testInfo.outputPath("graph-list.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Graphe", exact: true }).click();
  await page.screenshot({
    path: testInfo.outputPath("graph-relations.png"),
    fullPage: true,
  });
  for (const width of [1440, 1024, 390]) {
    await page.setViewportSize({ width, height: 900 });
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <=
          document.documentElement.clientWidth,
      ),
    ).toBe(true);
  }
  expect(errors).toEqual([]);
});
