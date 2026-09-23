import { expect, test } from "@playwright/test";

// Real API and disposable PostgreSQL, deterministic [FICTIF] engine only.
// The response is deliberately lost after server completion: reload must read
// its receipt and open the same draft without invoking generation a second time.
test("retrouve le brouillon de l’agent après réponse perdue puis le révise avant validation", async ({
  page,
  request,
}) => {
  const api = process.env.AI_CENTER_REAL_E2E_API_URL ?? "http://127.0.0.1:4617";
  const created = await request.post(`${api}/api/projects`, {
    headers: { "Idempotency-Key": crypto.randomUUID() },
    data: {
      name: `[FICTIF] Livrables ${crypto.randomUUID().slice(0, 8)}`,
      objective: "Préparer le portail de l’équipe et conserver les sources.",
    },
  });
  expect(created.status()).toBe(200);
  const project = await created.json();
  const sessionResponse = await request.post(
    `${api}/api/projects/${project.public_id}/sessions`,
    {
      headers: { "Idempotency-Key": crypto.randomUUID() },
      data: { node_key: "product", title: "[FICTIF] Préparer les tickets" },
    },
  );
  expect(sessionResponse.status()).toBe(200);
  const session = await sessionResponse.json();
  let generationCalls = 0;
  let artifactId = "";
  await page.route(
    `**/api/projects/${project.public_id}/artifacts/generate`,
    async (route) => {
      generationCalls++;
      const response = await route.fetch();
      expect(response.status()).toBe(200);
      const result = await response.json();
      artifactId = result.artifact.public_id;
      await route.abort("failed");
    },
  );
  await page.goto(
    `/projects/${project.public_id}/artifacts?create=1&session=${session.session.public_id}`,
  );
  await page.getByLabel("Type de livrable").selectOption("product_tickets");
  await page
    .getByLabel("Ce que le document doit préparer")
    .fill("[FICTIF] Préparer les tickets d’accès au portail.");
  await page
    .getByRole("button", { name: "Générer le brouillon avec l’agent" })
    .click();
  await expect(
    page.getByText("Le brouillon n’a pas pu être confirmé"),
  ).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Ce que le document doit préparer")).toHaveValue(
    "[FICTIF] Préparer les tickets d’accès au portail.",
  );
  const recovered = page.getByRole("link", {
    name: "Ouvrir le brouillon retrouvé",
  });
  await expect(recovered).toHaveAttribute("href", `/artifacts/${artifactId}`);
  await recovered.click();
  await expect(
    page.getByRole("heading", { name: "[FICTIF] Brouillon product_tickets" }),
  ).toBeVisible();
  expect(generationCalls).toBe(1);
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page
    .getByLabel("Description du ticket 1", { exact: true })
    .fill("[FICTIF] Créer un accès sur invitation, relu par le PM.");
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  await expect(page.getByText(/Version 2 · Brouillon/)).toBeVisible();
  await page.getByRole("button", { name: "Valider cette version" }).click();
  await expect(page.getByText(/Version 3 · Validée/)).toBeVisible();
  const detail = await request.get(`${api}/api/artifacts/${artifactId}`);
  expect(detail.status()).toBe(200);
  const document = await detail.json();
  expect(
    document.current_version.structured_content.draft.tickets[0].description,
  ).toBe("[FICTIF] Créer un accès sur invitation, relu par le PM.");
  expect(document.current_version.body_markdown).toContain("relu par le PM");
  expect(document.current_version.sources).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        kind: "session",
        public_id: session.session.public_id,
      }),
    ]),
  );
});
