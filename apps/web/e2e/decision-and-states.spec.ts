import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("attribue un signal à son projet et résout par révision", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page);

  await page.goto("/insights");
  await expect(page.getByText("AI Center").last()).toBeVisible();
  await page.getByRole("link", { name: /Rétention contradictoire/ }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${ids.project}/insights/${ids.insight}$`),
  );

  await page
    .getByLabel("Justification de la décision")
    .fill("La règle Produit doit être précisée et versionnée.");
  await page.getByRole("button", { name: "Réviser pour résoudre" }).click();
  await page
    .getByLabel("Nouvelle vérité du projet")
    .fill("Les données sont conservées 90 jours puis supprimées.");
  await page.getByRole("button", { name: "Réviser et résoudre" }).click();
  await expect(page.getByText("resolved")).toBeVisible();

  const request = state.requests.find((entry) =>
    entry.url().endsWith(`/insights/${ids.insight}/resolve`),
  );
  expect(request).toBeDefined();
  expect(request?.postDataJSON()).toMatchObject({
    expected_graph_version: 4,
    mutations: [
      {
        kind: "revise_knowledge",
        knowledge_public_id: ids.knowledge,
        expected_version_public_id: ids.version,
        statement: "Les données sont conservées 90 jours puis supprimées.",
      },
    ],
  });
  expect(errors).toEqual([]);
});

test("distingue une panne de couverture du reste de la page", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { coverageFailure: true });

  await page.goto(`/projects/${ids.project}/deliverables`);
  await expect(
    page.getByText("La couverture n’a pas pu être calculée"),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Livrables et couverture" }),
  ).toBeVisible();
  expect(
    errors.every((error) => error.includes("503 (Service Unavailable)")),
  ).toBe(true);
});

test("rend un vrai état 404 sans redirection silencieuse", async ({ page }) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page);

  await page.goto("/route-inconnue");
  await expect(
    page.getByRole("heading", { name: "Cette page n’existe pas" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/route-inconnue$/);
  expect(errors).toEqual([]);
});

test("refuse une session qui appartient à un autre projet", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page);

  await page.goto(`/projects/${ids.otherProject}/sessions/${ids.techSession}`);
  await expect(
    page.getByRole("heading", { name: "Session introuvable dans ce projet" }),
  ).toBeVisible();
  expect(errors.every((error) => error.includes("404 (Not Found)"))).toBe(true);
});
