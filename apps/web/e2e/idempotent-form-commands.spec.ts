import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("fige la création et rejoue exactement la même commande", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    projectCreateFailsOnce: true,
    projectCreateDelayMs: 250,
  });

  await page.goto("/projects/new");
  const name = page.getByLabel("Nom du projet");
  const objective = page.getByLabel("Objectif initial");
  await name.fill("Context Proof");
  await objective.fill("Prouver le contexte de bout en bout");
  await page.getByRole("button", { name: "Créer les scopes" }).click();

  await expect(name).toBeDisabled();
  await expect(objective).toBeDisabled();
  await expect(page.getByText("Le projet n’a pas été créé")).toBeVisible();
  await expect(name).toBeEnabled();
  await expect(objective).toBeEnabled();
  await expect(name).toHaveValue("Context Proof");
  await expect(objective).toHaveValue("Prouver le contexte de bout en bout");

  await page.getByRole("button", { name: "Réessayer" }).click();
  await expect(page).toHaveURL(`/projects/${ids.project}`);

  const attempts = state.requests.filter(
    (request) =>
      new URL(request.url()).pathname === "/api/projects" &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(attempts[1]?.postDataJSON()).toEqual(attempts[0]?.postDataJSON());
  expect(state.projectCreateAttempts).toBe(2);
  expect(errors.every((error) => error.includes("503"))).toBe(true);
});

test("crée une nouvelle commande après édition d’une création échouée", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { projectCreateFailsOnce: true });

  await page.goto("/projects/new");
  const name = page.getByLabel("Nom du projet");
  const objective = page.getByLabel("Objectif initial");
  await name.fill("Context Proof");
  await objective.fill("Objectif initial");
  await page.getByRole("button", { name: "Créer les scopes" }).click();
  await expect(page.getByText("Le projet n’a pas été créé")).toBeVisible();

  await objective.fill("Objectif corrigé");
  await expect(page.getByText("Le projet n’a pas été créé")).toHaveCount(0);
  await page.getByRole("button", { name: "Créer les scopes" }).click();
  await expect(page).toHaveURL(`/projects/${ids.project}`);

  const attempts = state.requests.filter(
    (request) =>
      new URL(request.url()).pathname === "/api/projects" &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).not.toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(attempts[0]?.postDataJSON()).toEqual({
    name: "Context Proof",
    objective: "Objectif initial",
  });
  expect(attempts[1]?.postDataJSON()).toEqual({
    name: "Context Proof",
    objective: "Objectif corrigé",
  });
  expect(errors.every((error) => error.includes("503"))).toBe(true);
});

test("crée une nouvelle commande message après édition d’un échec", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    messageFailsOnce: true,
    messageDelayMs: 250,
  });
  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);

  const composer = page.getByLabel("Message à l’agent tech");
  await composer.fill("Premier contenu");
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(composer).toBeDisabled();
  await expect(page.getByText("Le message n’a pas été envoyé")).toBeVisible();
  await expect(composer).toBeEnabled();
  await expect(composer).toHaveValue("Premier contenu");

  await composer.fill("Contenu corrigé");
  await expect(page.getByText("Le message n’a pas été envoyé")).toHaveCount(0);
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(composer).toHaveValue("");

  const attempts = state.requests.filter(
    (request) =>
      request.url().endsWith(`/sessions/${ids.techSession}/messages`) &&
      request.method() === "POST",
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0]?.headers()["idempotency-key"]).not.toBe(
    attempts[1]?.headers()["idempotency-key"],
  );
  expect(attempts[0]?.postDataJSON()).toEqual({
    content: "Premier contenu",
    client_message_id: attempts[0]?.headers()["idempotency-key"],
  });
  expect(attempts[1]?.postDataJSON()).toEqual({
    content: "Contenu corrigé",
    client_message_id: attempts[1]?.headers()["idempotency-key"],
  });
  expect(state.messageProviderAttempts).toBe(2);
  expect(state.messageMutations).toBe(1);
  expect(errors.every((error) => error.includes("503"))).toBe(true);
});
