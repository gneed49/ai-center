import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

// Real API and disposable DB, deterministic agent only. No publication or billing.
test("prépare des tickets avec un agent puis conserve leurs sources après relecture", async ({
  page,
  request,
}, testInfo) => {
  const api = process.env.AI_CENTER_REAL_E2E_API_URL ?? "http://127.0.0.1:4617";
  const post = async (path: string, data: unknown) => {
    const response = await request.post(`${api}${path}`, {
      headers: { "Idempotency-Key": crypto.randomUUID() },
      data,
    });
    expect(response.status()).toBe(200);
    return response.json();
  };
  await post("/api/company/setup", {
    name: "[FICTIF] Équipe des livrables",
    description: "Parcours déterministe, sans appel externe.",
  });
  const project = await post("/api/projects", {
    name: `[FICTIF] Invitations ${crypto.randomUUID().slice(0, 8)}`,
    objective: "Préparer les invitations de l’entreprise.",
  });
  const source = await post(`/api/projects/${project.public_id}/artifacts`, {
    artifact_type: "specification",
    title: "[FICTIF] Règle d’invitation",
    body_markdown:
      "Une invitation permet de rejoindre l’entreprise comme lecteur.",
    structured_content: {},
    sources: [],
  });
  const validated = await post(
    `/api/artifacts/${source.artifact.public_id}/validate`,
    {
      expected_version_id: source.current_version.public_id,
    },
  );
  const session = await post(`/api/projects/${project.public_id}/sessions`, {
    node_key: "product",
    title: "[FICTIF] Préparer les invitations",
  });
  const sessionId = session.session.public_id;
  await page.goto(`/projects/${project.public_id}/sessions/${sessionId}`);
  await page
    .getByLabel("Message à l’agent product")
    .fill("Préparons les invitations de lecteurs.");
  const replied = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().endsWith("/messages"),
  );
  await page.getByRole("button", { name: "Envoyer", exact: true }).click();
  expect((await replied).status()).toBe(200);
  await page.goto(
    `/projects/${project.public_id}/artifacts?create=1&session=${sessionId}`,
  );
  await page
    .getByLabel("Type de livrable", { exact: true })
    .selectOption("product_tickets");
  await expect(
    page.getByRole("region", { name: "Préparation avec un agent" }),
  ).toContainText("tickets produit");
  await expect(page.getByLabel("Conversation de départ")).toHaveValue(
    sessionId,
  );
  await page
    .getByLabel("Ce que le document doit préparer")
    .fill(
      "[FICTIF] Préparer les tickets d’invitation selon la règle validée, avec des critères vérifiables.",
    );
  const generatedResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().endsWith("/artifacts/generate"),
  );
  await page
    .getByRole("button", { name: "Générer le brouillon avec l’agent" })
    .click();
  const response = await generatedResponse;
  expect(response.status()).toBe(200);
  const generated = await response.json();
  const artifactId = generated.artifact.public_id;
  await expect(page).toHaveURL(new RegExp(`/artifacts/${artifactId}$`));
  await expect(page.getByText(/Version 1 · Brouillon/)).toBeVisible();
  expect(
    generated.current_version.structured_content.draft.tickets,
  ).toHaveLength(1);
  expect(generated.current_version.sources).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        kind: "artifact_version",
        public_id: validated.current_version.public_id,
      }),
    ]),
  );
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page
    .getByLabel("Titre du ticket 1", { exact: true })
    .fill("[FICTIF] Inviter un lecteur");
  await page
    .getByLabel("Critères d’acceptation du ticket 1 · un par ligne")
    .fill(
      "[FICTIF] Une invitation acceptée donne un accès lecteur.\n[FICTIF] Le lecteur ne peut pas modifier un projet.",
    );
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  await expect(page.getByText(/Version 2 · Brouillon/)).toBeVisible();
  await page.getByRole("button", { name: "Valider cette version" }).click();
  await expect(page.getByText(/Version 3 · Validée/)).toBeVisible();
  await page.reload();
  await expect(page.getByText(/Version 3 · Validée/)).toBeVisible();
  const currentResponse = await request.get(
    `${api}/api/artifacts/${artifactId}`,
  );
  expect(currentResponse.status()).toBe(200);
  const current = await currentResponse.json();
  const ticket = current.current_version.structured_content.draft.tickets[0];
  expect(ticket.title).toBe("[FICTIF] Inviter un lecteur");
  expect(ticket.acceptance_criteria).toHaveLength(2);
  expect(current.current_version.body_markdown).toContain(ticket.title);
  expect(current.current_version.body_markdown).toContain(
    ticket.acceptance_criteria[1],
  );
  expect(current.current_version.sources).toEqual(
    generated.current_version.sources,
  );
  const graphResponse = await request.get(
    `${api}/api/projects/${project.public_id}/graph`,
  );
  expect(graphResponse.status()).toBe(200);
  const graph = await graphResponse.json();
  expect(
    graph.edges.some(
      (edge: { target_public_id: string; source_public_id: string }) =>
        edge.source_public_id === current.current_version.public_id &&
        edge.target_public_id === validated.current_version.public_id,
    ),
  ).toBe(true);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: testInfo.outputPath("real-agent-tickets.png"),
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    accessibility.violations.filter((issue) =>
      ["critical", "serious"].includes(issue.impact ?? ""),
    ),
  ).toEqual([]);
});
