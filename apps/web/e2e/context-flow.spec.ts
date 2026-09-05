import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("restaure le dernier handoff et son ContextPack après reload", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page);

  await page.goto(`/projects/${ids.project}/handoff`);
  await expect(
    page.getByRole("heading", { name: "Handoff Produit → Tech" }),
  ).toBeVisible();
  await expect(page.getByText("Handoff terminé et courant")).toBeVisible();
  await expect(
    page.getByRole("region", { name: "ContextPack version 2" }),
  ).toBeVisible();
  await expect(
    page.getByText("GitHub reste la source canonique"),
  ).toBeVisible();

  await page.reload();
  await expect(page.getByText("Handoff terminé et courant")).toBeVisible();
  expect(
    state.requests.filter((request) =>
      request.url().endsWith(`/projects/${ids.project}/handoffs/latest`),
    ).length,
  ).toBeGreaterThanOrEqual(2);
  expect(errors).toEqual([]);
});

test("recompile un pack stale avant de créer le handoff", async ({ page }) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    packStatus: "stale",
    packGraphVersion: 3,
  });

  await page.goto(`/projects/${ids.project}/handoff`);
  await expect(page.getByText("Obsolète").first()).toBeVisible();
  await page
    .getByRole("button", { name: "Recompiler et ouvrir une session Tech" })
    .click();

  await expect(page.getByText("Handoff terminé et courant")).toBeVisible();
  const mutations = state.requests.filter(
    (request) => request.method() === "POST",
  );
  expect(mutations.map((request) => new URL(request.url()).pathname)).toEqual([
    `/api/projects/${ids.project}/context-packs`,
    `/api/projects/${ids.project}/handoffs`,
  ]);
  expect(mutations[0]?.headers()["idempotency-key"]).toBeTruthy();
  expect(mutations[1]?.headers()["idempotency-key"]).toBeTruthy();
  expect(mutations[0]?.headers()["idempotency-key"]).not.toBe(
    mutations[1]?.headers()["idempotency-key"],
  );
  expect(errors).toEqual([]);
});

test("bloque une session Tech alimentée par un pack stale", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { packStatus: "stale", packGraphVersion: 3 });

  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);
  await expect(
    page.getByRole("heading", { name: "Session Tech bloquée" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Recompiler le ContextPack" }),
  ).toHaveAttribute("href", `/projects/${ids.project}/handoff`);
  await expect(page.getByLabel("Message à l’agent tech")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("bloque une session Tech sans ContextPack", async ({ page }) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { sessionHasPack: false });

  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);
  await expect(
    page.getByRole("heading", { name: "Session Tech bloquée" }),
  ).toBeVisible();
  await expect(page.getByLabel("Message à l’agent tech")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("refuse un gate passé sur une ancienne version du graphe", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, {
    gateGraphVersion: 3,
    latestHandoff: false,
  });

  await page.goto(`/projects/${ids.project}/handoff`);
  await expect(page.getByText("À réévaluer")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Évaluer le gate" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: /Compiler et ouvrir/ }),
  ).toHaveCount(0);
  expect(errors.every((error) => error.includes("404 (Not Found)"))).toBe(true);
});

test("préserve le message et réutilise la clé lors d’un retry", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    messageFailsOnce: true,
    messageDelayMs: 250,
  });
  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);

  const composer = page.getByLabel("Message à l’agent tech");
  await composer.fill("Prépare le plan de preuve GitHub");
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(composer).toBeDisabled();
  await expect(page.getByText("Le message n’a pas été envoyé")).toBeVisible();
  await expect(composer).toBeEnabled();
  await expect(composer).toHaveValue("Prépare le plan de preuve GitHub");
  await page.getByRole("button", { name: "Réessayer" }).click();
  await expect(composer).toHaveValue("");

  const attempts = state.requests.filter(
    (request) =>
      request.url().endsWith(`/sessions/${ids.techSession}/messages`) &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(attempts[0]?.postDataJSON().client_message_id).toBe(
    attempts[0]?.headers()["idempotency-key"],
  );
  expect(attempts[1]?.postDataJSON().client_message_id).toBe(
    attempts[0]?.postDataJSON().client_message_id,
  );
  expect(attempts[1]?.postDataJSON()).toEqual(attempts[0]?.postDataJSON());
  expect(state.messageProviderAttempts).toBe(2);
  expect(state.messageMutations).toBe(1);
  expect(
    errors.every((error) => error.includes("503 (Service Unavailable)")),
  ).toBe(true);
});

test("rejoue le succès sans mutation après perte de la réponse", async ({
  page,
}) => {
  const failedRequests: string[] = [];
  page.on("requestfailed", (request) => {
    if (request.url().endsWith(`/sessions/${ids.techSession}/messages`))
      failedRequests.push(request.failure()?.errorText ?? "unknown");
  });
  const state = await installMockApi(page, { messageResponseLostOnce: true });
  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);

  const composer = page.getByLabel("Message à l’agent tech");
  await composer.fill("Prépare le plan de preuve GitHub");
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(page.getByText("Le message n’a pas été envoyé")).toBeVisible();
  await expect(composer).toHaveValue("Prépare le plan de preuve GitHub");
  await page.getByRole("button", { name: "Réessayer" }).click();
  await expect(composer).toHaveValue("");

  const attempts = state.requests.filter(
    (request) =>
      request.url().endsWith(`/sessions/${ids.techSession}/messages`) &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(failedRequests).toHaveLength(1);
  expect(state.messageProviderAttempts).toBe(1);
  expect(state.messageMutations).toBe(1);
});

test("rejoue une erreur permanente sans réexécuter la commande", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { messagePermanentFailure: true });
  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);

  const composer = page.getByLabel("Message à l’agent tech");
  await composer.fill("Commande invalide mais stable");
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(page.getByText("Le message n’a pas été envoyé")).toBeVisible();
  await page.getByRole("button", { name: "Réessayer" }).click();
  await expect(page.getByText("Le message n’a pas été envoyé")).toBeVisible();
  await expect(composer).toHaveValue("Commande invalide mais stable");

  const attempts = state.requests.filter(
    (request) =>
      request.url().endsWith(`/sessions/${ids.techSession}/messages`) &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(state.messageProviderAttempts).toBe(1);
  expect(state.messageMutations).toBe(0);
  expect(errors.every((error) => error.includes("422"))).toBe(true);
});

test("réutilise la clé du gate après une erreur transitoire", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { gateFailsOnce: true });
  await page.goto(`/projects/${ids.project}`);

  await page.getByRole("button", { name: "Évaluer maintenant" }).click();
  await expect(page.getByText("Le gate n’a pas pu être évalué")).toBeVisible();
  await page.getByRole("button", { name: "Réessayer" }).click();
  await expect(page.getByText("Le gate n’a pas pu être évalué")).toHaveCount(0);

  const attempts = state.requests.filter(
    (request) =>
      request.url().endsWith(`/gates/product-ready/evaluate`) &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(state.gateAttempts).toBe(2);
  expect(
    errors.every((error) => error.includes("503 (Service Unavailable)")),
  ).toBe(true);
});
