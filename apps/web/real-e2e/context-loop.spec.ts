import { expect, test } from "@playwright/test";

test("traverse le vrai backend déterministe jusqu’au plan, à sa couverture et à son historique", async ({
  page,
}, testInfo) => {
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

  const projectPath = new URL(page.url()).pathname.replace(/\/handoff$/, "");
  await page.goto(`${projectPath}/deliverables`);
  await expect(
    page.getByRole("heading", { name: "Livrables et couverture" }),
  ).toBeVisible();
  const planCard = page.locator("aside > div").filter({
    has: page.getByRole("heading", {
      name: "Plan de livraison Tech",
      exact: true,
    }),
  });
  const generated = page.waitForResponse(
    (response) =>
      response.url().endsWith("/deliverables/technical-plan") &&
      response.request().method() === "POST",
  );
  await planCard.getByRole("button", { name: "Générer" }).click();
  const generatedResponse = await generated;
  expect(generatedResponse.status()).toBe(200);
  const plan = await generatedResponse.json();
  expect(plan.coverage_status).toBe("missing");
  expect(plan.content.coverage_assessment.requirements).toHaveLength(1);
  expect(plan.content.coverage_assessment.requirements[0].assessment).toBe(
    "planned",
  );
  await page.getByRole("link", { name: /Technical Delivery Plan/ }).click();
  await expect(
    page.getByRole("heading", { name: "Technical Delivery Plan", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Exigences reliées", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByText(
        "Aucune preuve externe validée n’est encore attachée à cette exigence.",
      )
      .first(),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Technical Delivery Plan", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("real-plan-coverage.png"),
    fullPage: true,
  });
  await page.goto(`${projectPath}/history`);
  await expect(
    page.getByRole("heading", { name: "Historique du projet" }),
  ).toBeVisible();
  const committed = page.getByText("deliverable · committed", { exact: true });
  await expect(committed).toHaveCount(1);
  await committed.locator("..").locator("summary").click();
  await expect(committed.locator("..").locator("pre")).toContainText(
    "coverage_assessment",
  );
  await expect(committed.locator("..").locator("pre")).toContainText(
    "Technical Delivery Plan",
  );
  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
  expect(requestFailures).toEqual([]);
});
