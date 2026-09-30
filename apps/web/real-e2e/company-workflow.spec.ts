import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { readFile } from "node:fs/promises";

// Real API and disposable PostgreSQL; all content is fictional. This test
// makes no model-provider, mail or work-tool HTTP call.
test("organise un livrable réel, ses versions et son graphe d’entreprise", async ({
  page,
  request,
}, testInfo) => {
  const api = process.env.AI_CENTER_REAL_E2E_API_URL ?? "http://127.0.0.1:4617";
  const headers = { "Idempotency-Key": crypto.randomUUID() };
  const setup = await request.post(`${api}/api/company/setup`, {
    headers,
    data: {
      name: "[FICTIF] Atelier Horizon",
      description: "Une équipe et ses projets partagés.",
    },
  });
  expect(setup.status()).toBe(200);
  const created = await request.post(`${api}/api/projects`, {
    headers: { "Idempotency-Key": crypto.randomUUID() },
    data: {
      name: `[FICTIF] Portail ${crypto.randomUUID().slice(0, 8)}`,
      objective: "Préparer le portail client avec des sources traçables.",
    },
  });
  expect(created.status()).toBe(200);
  const project = await created.json();
  const projectId = project.public_id;
  expect(projectId).toMatch(/^[\da-f-]{36}$/);
  await page.goto(`/projects/${projectId}/artifacts?create=1`);
  await page
    .getByLabel("Titre du livrable")
    .fill("[FICTIF] Spécification du portail");
  await page
    .getByLabel("Contenu", { exact: true })
    .fill(
      "# Accès au portail\n\nL’accès nécessite une invitation de l’entreprise.",
    );
  await page.getByRole("button", { name: "Créer le brouillon" }).click();
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Spécification du portail" }),
  ).toBeVisible();
  const artifactPath = new URL(page.url()).pathname;
  await page.getByRole("button", { name: "Valider cette version" }).click();
  await expect(page.getByText(/Version 2 · Validée/)).toBeVisible();
  await page.reload();
  await expect(page.getByText(/Version 2 · Validée/)).toBeVisible();
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page
    .getByLabel("Contenu", { exact: true })
    .fill(
      "# Accès au portail\n\nL’accès nécessite une invitation de l’entreprise. Un lecteur consulte sans modifier.",
    );
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  await expect(page.getByText(/Version 3 · Brouillon/)).toBeVisible();
  await page.getByRole("button", { name: "Valider cette version" }).click();
  await expect(page.getByText(/Version 4 · Validée/)).toBeVisible();
  await page
    .getByLabel("Version affichée")
    .selectOption({ label: "Version 2 · validée" });
  await expect(
    page.getByText("Vous consultez une version historique.", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByText("Un lecteur consulte sans modifier.", { exact: false }),
  ).toHaveCount(0);
  const artifactId = artifactPath.split("/").at(-1);
  const detail = await request.get(`${api}/api/artifacts/${artifactId}`);
  expect(detail.status()).toBe(200);
  const current = await detail.json();
  expect(current.current_version.version).toBe(4);
  await page
    .getByRole("link", { name: "Graphe du projet", exact: true })
    .click();
  await expect(page).toHaveURL(new RegExp(`/graph\\?project=${projectId}$`));
  const node = page.getByRole("button", {
    name: `[FICTIF] Spécification du portail, version 4, Livrable, ${project.name}`,
    exact: true,
  });
  await expect(node).toBeVisible();
  await node.click();
  await expect(
    page.getByRole("link", { name: "Ouvrir la source" }),
  ).toHaveAttribute(
    "href",
    new RegExp(`${artifactId}.*${current.current_version.public_id}`),
  );
  await page.screenshot({
    path: testInfo.outputPath("real-project-graph.png"),
    fullPage: true,
  });
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    accessibility.violations.filter(
      (issue) => issue.impact === "critical" || issue.impact === "serious",
    ),
  ).toEqual([]);
  await page.goto(`/projects/${projectId}`);
  const pendingDownload = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Exporter ce projet", exact: true })
    .click();
  const downloaded = await pendingDownload;
  const exported = JSON.parse(
    await readFile((await downloaded.path())!, "utf8"),
  );
  expect(exported).toMatchObject({
    format: "ai-center-project-data-v1",
    project_public_id: projectId,
    complete: true,
  });
  expect(exported.data.artifact_document_versions).toHaveLength(4);
  expect(
    exported.data.artifact_document_versions.some(
      (version: { public_id: string }) =>
        version.public_id === current.current_version.public_id,
    ),
  ).toBe(true);
  expect(exported.data).not.toHaveProperty("work_tool_connections");
  await page.goto("/company");
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Atelier Horizon" }),
  ).toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("real-company-team.png"),
    fullPage: true,
  });
});
