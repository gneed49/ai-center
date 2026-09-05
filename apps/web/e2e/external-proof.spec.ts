import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("observe une PR, valide la preuve puis la rend stale après changement de SHA", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { externalProof: true });

  await page.goto(`/projects/${ids.project}/deliverables/${ids.deliverable}`);
  await expect(
    page.getByRole("heading", { name: "Preuves GitHub observées" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Aucune référence GitHub n’est encore rattachée à ce projet.",
    ),
  ).toBeVisible();

  await page
    .getByLabel(
      "URL canonique du dépôt, de la pull request ou du commit GitHub",
    )
    .fill("https://github.com/gneed49/ai-center/pull/42");
  await page.getByRole("button", { name: "Observer la référence" }).click();
  await expect(page.getByText("PR #42 — preuve ContextPack")).toBeVisible();
  await expect(page.getByText(/head b{12}/)).toBeVisible();

  await page.getByRole("button", { name: "Créer la preuve candidate" }).click();
  await expect(page.getByText("candidate", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Valider humainement" }).click();
  await expect(page.getByText("valid", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Rafraîchir" }).click();
  await expect(page.getByText(/head c{12}/)).toBeVisible();
  await expect(page.getByText("stale", { exact: true })).toBeVisible();

  expect(state.evidenceStatus).toBe("stale");
  const mutations = state.requests.filter(
    (request) =>
      request.method() === "POST" &&
      request.url().includes("external-references"),
  );
  expect(mutations).toHaveLength(4);
  const keys = mutations.map((request) => request.headers()["idempotency-key"]);
  expect(keys.every((key) => /^[0-9a-f-]{36}$/.test(key ?? ""))).toBe(true);
  expect(new Set(keys).size).toBe(keys.length);
  expect(errors).toEqual([]);
});

test("persiste une preuve rejetée et retourne son état", async ({ page }) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, { externalProof: true });

  await page.goto(`/projects/${ids.project}/deliverables/${ids.deliverable}`);
  await page
    .getByLabel(
      "URL canonique du dépôt, de la pull request ou du commit GitHub",
    )
    .fill("https://github.com/gneed49/ai-center/pull/42");
  await page.getByRole("button", { name: "Observer la référence" }).click();
  await page.getByRole("button", { name: "Créer la preuve candidate" }).click();
  await page.getByRole("button", { name: "Rejeter" }).click();

  await expect(page.getByText("rejected", { exact: true })).toBeVisible();
  expect(state.evidenceStatus).toBe("rejected");
  expect(errors).toEqual([]);
});

for (const kind of ["repository", "commit"] as const) {
  test(`importe une référence ${kind} et expose les actions adaptées`, async ({
    page,
  }, testInfo) => {
    const errors = captureConsoleErrors(page);
    const state = await installMockApi(page, { externalProof: true });
    const url =
      kind === "repository"
        ? "https://github.com/gneed49/ai-center"
        : `https://github.com/gneed49/ai-center/commit/${"b".repeat(40)}`;
    await page.goto(`/projects/${ids.project}/deliverables/${ids.deliverable}`);
    await page
      .getByLabel(
        "URL canonique du dépôt, de la pull request ou du commit GitHub",
      )
      .fill(url);
    await page.getByRole("button", { name: "Observer la référence" }).click();
    await expect(
      page.getByRole("link", {
        name:
          kind === "repository"
            ? "gneed49/ai-center"
            : `Commit ${"b".repeat(40)}`,
        exact: true,
      }),
    ).toBeVisible();
    expect(state.externalReferenceUrl).toBe(url);
    if (kind === "repository") {
      await expect(
        page.getByText("Un dépôt décrit une source mutable.", { exact: false }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: "Créer la preuve candidate" }),
      ).toHaveCount(0);
    } else {
      await expect(page.getByText(/head b{12}/)).toBeVisible();
      await page
        .getByRole("button", { name: "Créer la preuve candidate" })
        .click();
      await expect(page.getByText("candidate", { exact: true })).toBeVisible();
    }
    await page
      .getByRole("heading", { name: "Preuves GitHub observées" })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: `/tmp/acp-github-${kind}-${testInfo.project.name}.png`,
    });
    expect(errors).toEqual([]);
  });
}

test("déclare le pack transmis et conserve la chaîne après réponse perdue et rechargement", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    externalProof: true,
    externalImportResponseLostOnce: true,
  });
  await page.goto(`/projects/${ids.project}/deliverables/${ids.deliverable}`);
  const pack = page.getByLabel(
    "ContextPack que j’ai transmis à l’outil externe",
  );
  await expect(pack).toHaveValue("");
  await pack.selectOption(ids.pack);
  await page
    .getByLabel(
      "URL canonique du dépôt, de la pull request ou du commit GitHub",
    )
    .fill("https://github.com/gneed49/ai-center/pull/42");
  await page.getByRole("button", { name: "Observer la référence" }).click();
  await page
    .getByRole("button", { name: "Réessayer avec la même commande" })
    .click();
  await expect(
    page.getByText("ContextPack transmis v2", { exact: false }),
  ).toBeVisible();
  expect(state.externalTracking).toHaveLength(1);
  const chain = state.externalTracking[0];
  const artifactId = chain.artifacts[0].public_id;
  const imports = state.requests.filter(
    (request) =>
      request.method() === "POST" &&
      request.url().endsWith(`/projects/${ids.project}/external-references`),
  );
  expect(imports).toHaveLength(2);
  expect(imports[0].headers()["idempotency-key"]).toBe(
    imports[1].headers()["idempotency-key"],
  );
  expect(imports[0].postDataJSON().tracking).toEqual({
    context_pack_id: ids.pack,
    transmission_confirmed: true,
  });
  const candidate = page.getByRole("button", {
    name: "Créer la preuve candidate",
  });
  await expect(candidate).toBeDisabled();
  await page
    .getByLabel("Artefact observé pour la preuve")
    .selectOption(artifactId);
  await candidate.click();
  await page.getByRole("button", { name: "Valider humainement" }).click();
  await expect(page.getByText("valid", { exact: true })).toBeVisible();
  expect(state.evidenceArtifactId).toBe(artifactId);
  await page.reload();
  await expect(page.getByText("valid", { exact: true })).toBeVisible();
  await page.getByText("Historique des observations (3)").click();
  await expect(
    page.getByText("Preuve validée humainement", { exact: true }),
  ).toBeVisible();
  expect(state.externalTracking[0].execution_public_id).toBe(
    chain.execution_public_id,
  );
  await page.getByRole("button", { name: "Rafraîchir" }).click();
  await expect(page.getByText("stale", { exact: true })).toBeVisible();
  const oldArtifact = page
    .getByLabel("Artefact observé pour la preuve")
    .locator(`option[value="${artifactId}"]`);
  await expect(oldArtifact).toHaveJSProperty("disabled", true);
  expect(state.externalTracking[0].artifacts).toHaveLength(2);
  await page
    .getByText("ContextPack transmis v2", { exact: false })
    .scrollIntoViewIfNeeded();
  await page.screenshot({
    path: `/tmp/acp-tracking-${testInfo.project.name}.png`,
  });
  // The intentional simulated 503 is an expected network diagnostic.
  expect(errors.filter((error) => !error.includes("503"))).toEqual([]);
});

for (const stale of [
  { packStatus: "stale" as const },
  { packGraphVersion: 3, graphVersion: 4 },
]) {
  test(`refuse un pack ${"packStatus" in stale ? "marqué obsolète" : "d’une ancienne révision"} dans la déclaration`, async ({
    page,
  }) => {
    await installMockApi(page, { externalProof: true, ...stale });
    await page.goto(`/projects/${ids.project}/deliverables/${ids.deliverable}`);
    const pack = page.getByLabel(
      "ContextPack que j’ai transmis à l’outil externe",
    );
    await expect(pack).toHaveValue("");
    await expect(pack.locator(`option[value="${ids.pack}"]`)).toBeDisabled();
  });
}
