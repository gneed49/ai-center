import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { fixtureCode } from "../src/test-fixtures/code-evidence";
import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("ouvre une tâche et le fichier exact depuis la liste du graphe au clavier", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { companySuite: true });
  const corpus = {
    ...fixtureCode.corpus,
    public_id: ids.pack,
    project_id: ids.project,
  };
  const selected = {
    ...fixtureCode.files[0],
    public_id: ids.reference,
    path: "src/second.ts",
    content_text: "// [FICTIF] Second fichier uniquement",
    line_count: 1,
  };
  const filePath = `/projects/${ids.project}/code?observation=${corpus.public_id}&file=${selected.public_id}`;
  const taskPath = `/projects/${ids.project}/sources/task/${ids.deliverable}`;
  await page.route("**/api/company/graph", (route) =>
    route.fulfill({
      json: {
        workspace_public_id: "01000000-0000-4000-8000-000000000001",
        project_public_id: null,
        source_graph_versions: [],
        truncated: false,
        limits: { max_nodes: 250, max_edges: 750 },
        edges: [],
        nodes: [
          {
            id: ids.deliverable,
            kind: "task",
            label: "[FICTIF] Tâche transmise",
            status: "pending",
            app_path: taskPath,
          },
          {
            id: ids.reference,
            kind: "external_reference",
            label: "[FICTIF] Second fichier",
            status: "code_read",
            app_path: filePath,
          },
        ].map((node) => ({
          ...node,
          project_public_id: ids.project,
          scope_kind: "project",
          source_url: null,
          version_public_id: null,
        })),
      },
    }),
  );
  await page.route(
    `**/api/projects/${ids.project}/sources/task/${ids.deliverable}`,
    (route) =>
      route.fulfill({
        json: {
          public_id: ids.deliverable,
          kind: "task",
          project_public_id: ids.project,
          project_name: "[FICTIF] Projet partagé",
          scope_kind: "project",
          title: "[FICTIF] Tâche transmise",
          status: "pending",
          version_number: null,
          recorded_at: fixtureCode.corpus.observed_at,
          content: {
            objective:
              "[FICTIF] Préparer les tickets techniques à partir de la spécification validée.",
          },
          links: [
            {
              label: "Contexte reçu",
              app_path: `/projects/${ids.project}/sources/context_pack/${ids.pack}`,
            },
          ],
        },
      }),
  );
  await page.route(`**/api/code-observations/${corpus.public_id}`, (route) =>
    route.fulfill({
      json: { corpus, files: [fixtureCode.files[0], selected] },
    }),
  );
  await page.goto("/graph");
  await page.getByRole("button", { name: "Liste", exact: true }).click();
  const task = page
    .getByRole("list", { name: "Éléments du graphe" })
    .getByRole("button", { name: /Tâche transmise/ });
  await task.focus();
  await page.keyboard.press("Enter");
  const open = page.getByRole("link", { name: "Ouvrir la source" });
  await expect(open).toHaveAttribute("href", taskPath);
  await open.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("region", { name: "Contenu de la source" }),
  ).toContainText("Préparer les tickets techniques");
  await expect(
    page.getByRole("link", { name: "Contexte reçu" }),
  ).toHaveAttribute(
    "href",
    `/projects/${ids.project}/sources/context_pack/${ids.pack}`,
  );
  await page.goto("/graph");
  await page.getByRole("button", { name: "Liste", exact: true }).click();
  await page
    .getByRole("list", { name: "Éléments du graphe" })
    .getByRole("button", { name: /Second fichier/ })
    .click();
  await page.getByRole("link", { name: "Ouvrir la source" }).click();
  await expect(page).toHaveURL(new RegExp(`file=${selected.public_id}$`));
  await expect(
    page.getByRole("heading", { name: selected.path }),
  ).toBeVisible();
  await expect(page.getByText(selected.content_text)).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: selected.path }),
  ).toBeVisible();
  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(axe.violations).toEqual([]);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: testInfo.outputPath("exact-source-mobile.png"),
    fullPage: true,
  });
  await page.goto(
    `/projects/${ids.project}/code?observation=${corpus.public_id}&file=${ids.otherProject}`,
  );
  await expect(page.getByRole("alert")).toContainText(
    "Fichier absent de cette observation",
  );
  await expect(
    page.getByRole("button", { name: "Copier la référence" }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});
