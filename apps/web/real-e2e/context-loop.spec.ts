import { expect, test } from "@playwright/test";

test("traverse le vrai backend déterministe jusqu’au handoff persistant", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  const requestFailures: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  page.on("requestfailed", (request) => {
    requestFailures.push(
      `${request.method()} ${new URL(request.url()).pathname}: ${request.failure()?.errorText ?? "unknown failure"}`,
    );
  });
  const projectName = `Context Proof ${crypto.randomUUID().slice(0, 8)}`;

  await page.goto("/projects/new");
  await page.getByLabel("Nom du projet").fill(projectName);
  await page
    .getByLabel("Objectif initial")
    .fill("Conserver des décisions validées, durables et traçables.");
  await page.getByRole("button", { name: "Créer les scopes" }).click();
  await expect(page.getByRole("heading", { name: projectName })).toBeVisible();

  const productScope = page
    .locator("article")
    .filter({ has: page.getByRole("heading", { name: "Produit" }) });
  await productScope.getByRole("button", { name: "Nouvelle session" }).click();
  await expect(page.getByLabel("Message à l’agent product")).toBeVisible();
  await page
    .getByLabel("Message à l’agent product")
    .fill("Les décisions validées restent consultables sans expiration.");
  await page.getByRole("button", { name: "Envoyer" }).click();

  const proposals = page.getByRole("button", {
    name: /^Sélectionner la proposition/,
  });
  await expect(proposals).toHaveCount(3);
  for (let index = 0; index < 3; index += 1) {
    await proposals.first().click();
  }
  await page.getByRole("button", { name: "Confirmer (3)" }).click();
  await expect(page.getByText("3 connaissance(s) confirmée(s)")).toBeVisible();

  await page.getByRole("link", { name: "Retour au projet" }).click();
  await expect(page.getByText("graphe v1").first()).toBeVisible();
  await page.getByRole("button", { name: "Évaluer maintenant" }).click();
  await expect(page.getByText("Produit prêt pour le handoff")).toBeVisible();

  await page.getByRole("link", { name: "Préparer le handoff" }).click();
  await page
    .getByRole("button", { name: "Compiler et ouvrir la session Tech" })
    .click();
  await expect(page.getByText("Handoff terminé et courant")).toBeVisible();
  await expect(
    page.getByRole("region", { name: /ContextPack version/ }),
  ).toBeVisible();

  await page.reload();
  await expect(page.getByText("Handoff terminé et courant")).toBeVisible();
  await expect(page.getByText(/Contexte inclus/)).toBeVisible();
  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
  expect(requestFailures).toEqual([]);
});
