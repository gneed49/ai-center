import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("crée une invitation sans transmettre son secret et contrôle la pause", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { companySuite: true });
  await page.goto("/company");
  await page.getByLabel("Repère pour votre suivi").fill("[FICTIF] Collègue");
  await page.getByLabel("Rôle accordé").selectOption("viewer");
  await page
    .getByRole("button", { name: "Créer le lien d’invitation" })
    .click();
  const link = await page.getByLabel("Lien de cette invitation").inputValue();
  expect(new URL(link).hash).toMatch(/^#token=[a-f0-9]{64}$/);
  const sent = state.requests.find(
    (request) =>
      request.url().endsWith("/api/team/invitations") &&
      request.method() === "POST",
  )!;
  expect(sent.postDataJSON().token_hash).toMatch(/^[a-f0-9]{64}$/);
  expect(sent.postData()).not.toContain(new URL(link).hash.slice(7));
  await page.getByRole("button", { name: "Révoquer l’invitation" }).click();
  await expect(
    page.getByRole("button", { name: "Révoquer l’invitation" }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Mettre l’automatisation en pause" })
    .click();
  await expect(
    page.getByText("Automatisation en pause", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Reprendre l’automatisation" })
    .click();
  await expect(
    page.getByText("Automatisation autorisée", { exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("révise et valide un livrable puis confirme et annule une publication en attente", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { companySuite: true });
  await page.goto(`/projects/${ids.project}/artifacts`);
  await page
    .getByRole("link", { name: "[FICTIF] Spécification atelier" })
    .click();
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page
    .getByLabel("Titre du livrable")
    .fill("[FICTIF] Révision approuvable");
  await page
    .getByLabel("Contenu", { exact: true })
    .fill("# [FICTIF] Exigence révisée\nUne décision explicite.");
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Révision approuvable" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Valider cette version" }).click();
  await page
    .getByRole("button", { name: "Préparer la publication de la version 4" })
    .click();
  await expect(
    page.getByRole("region", { name: "Confirmation de publication" }),
  ).toContainText("fiction/example");
  await page
    .getByRole("button", { name: "Confirmer la publication", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Annuler la demande en attente" })
    .click();
  await expect(
    page.getByText("Annulée", { exact: true }).first(),
  ).toBeVisible();
  const sent = state.requests.find(
    (request) =>
      request.url().includes("/artifacts/fixture-artifact/publications") &&
      request.method() === "POST",
  )!;
  expect(sent.postDataJSON()).toMatchObject({
    expected_provider: "github",
    expected_target_id: "fiction/example",
  });
  expect(errors).toEqual([]);
});

test("consulte les lignes observées d’un commit sans débordement ni défaut d’accessibilité", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { companySuite: true });
  await page.goto(`/projects/${ids.project}/code`);
  await expect(
    page.getByRole("heading", { name: "fiction/example" }),
  ).toBeVisible();
  await page.getByLabel("Première ligne").fill("2");
  await expect(
    page.getByRole("link", { name: "Voir ces lignes sur GitHub" }),
  ).toHaveAttribute("href", /\/blob\/a{40}\/src\/fixture.ts#L2-L3$/);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(axe.violations).toEqual([]);
  expect(errors).toEqual([]);
});
