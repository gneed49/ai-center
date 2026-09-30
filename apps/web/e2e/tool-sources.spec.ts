import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { captureConsoleErrors, ids } from "./mock-api";
import { installSourceApi, sourceIds } from "./tool-source-mock-api";

async function checkLayout(page: Page, screenshot: string) {
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  expect(await page.locator("vite-error-overlay").count()).toBe(0);
  const audit = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.map((v) => ({
      id: v.id,
      nodes: v.nodes.map((n) => n.target),
    })),
  ).toEqual([]);
  await page.screenshot({ path: screenshot, fullPage: true });
}

for (const provider of ["linear", "notion"] as const) {
  test(`ajoute une source ${provider} après partage explicite et conserve le reçu au rechargement`, async ({
    page,
  }, testInfo) => {
    const errors = captureConsoleErrors(page);
    const api = await installSourceApi(page);
    await page.goto(`/projects/${ids.project}/sources`);
    await expect(
      page.getByRole("heading", { name: "Sources du projet" }),
    ).toBeVisible();
    const form = page.getByRole("region", {
      name: "Ajouter une source",
      exact: true,
    });
    await form
      .getByRole("combobox", { name: "Outil", exact: true })
      .selectOption(provider);
    await form
      .getByLabel("Connexion autorisée")
      .selectOption(
        provider === "linear"
          ? sourceIds.linearConnection
          : sourceIds.notionConnection,
      );
    await form
      .getByLabel("Lien de la page ou du ticket")
      .fill(
        provider === "linear"
          ? "PROD-42"
          : "https://www.notion.so/70000000000040008000000000000001",
      );
    const submit = form.getByRole("button", {
      name: "Lire et ajouter la source",
    });
    await expect(submit).toBeDisabled();
    await page.setViewportSize({ width: 390, height: 844 });
    await checkLayout(
      page,
      `/tmp/t17-add-${provider}-${testInfo.project.name}.png`,
    );
    expect(api.reads).toBe(0);
    await form.getByRole("checkbox").check();
    await form.getByLabel("Connexion autorisée").selectOption("");
    await expect(form.getByRole("checkbox")).not.toBeChecked();
    await form
      .getByLabel("Connexion autorisée")
      .selectOption(
        provider === "linear"
          ? sourceIds.linearConnection
          : sourceIds.notionConnection,
      );
    await form.getByRole("checkbox").check();
    await submit.click();
    await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
    const sent = api.requests.find((r) => r.method() === "POST")!;
    expect(sent.postDataJSON()).toMatchObject({
      confirm_scope_sharing: true,
      provider,
    });
    expect(api.reads).toBe(1);
    await page.reload();
    await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
    expect(api.requests.filter((r) => r.method() === "POST")).toHaveLength(1);
    await page
      .getByRole("link", { name: "Ouvrir la lecture de cette demande" })
      .click();
    await expect(page).toHaveURL(`/source-observations/${sourceIds.original}`);
    await expect(
      page.getByRole("heading", { name: "Lecture exacte de la source" }),
    ).toBeVisible();
    await expect(
      page.getByRole("article", { name: "Contenu de la lecture" }),
    ).toContainText(
      provider === "linear"
        ? "Les crédits acquis restent disponibles"
        : "Le cahier Notion reste",
    );
    await checkLayout(
      page,
      `/tmp/t17-${provider}-${testInfo.project.name}.png`,
    );
    expect(errors).toEqual([]);
  });
}

test("retrouve une réponse perdue sans nouvelle lecture et sans erreur contradictoire", async ({
  page,
}) => {
  const errors: string[] = [];
  const failures: string[] = [];
  const lostUrl = `http://127.0.0.1:4317/api/projects/${ids.project}/tool-sources`;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("requestfailed", (request) => failures.push(request.url()));
  page.on("console", (message) => {
    if (message.type() !== "error") return;
    const expected =
      (message.location().url === lostUrl &&
        message.text() === "Failed to load resource: net::ERR_FAILED") ||
      message.text() ===
        `[JavaScript Error: "Cross-Origin Request Blocked: The Same Origin Policy disallows reading the remote resource at ${lostUrl}. (Reason: CORS request did not succeed). Status code: (null)."]`;
    if (!expected) errors.push(message.text());
  });
  const api = await installSourceApi(page, { lostResponse: true });
  await page.goto(`/projects/${ids.project}/sources`);
  await page
    .getByLabel("Connexion autorisée")
    .selectOption(sourceIds.linearConnection);
  await page.getByLabel("Lien de la page ou du ticket").fill("PROD-42");
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Lire et ajouter la source" }).click();
  await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "Sources rattachées" }),
  ).toContainText("[FICTIF] Durée des crédits");
  await page.reload();
  await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
  expect(api.reads).toBe(1);
  expect(api.requests.filter((r) => r.method() === "POST")).toHaveLength(1);
  expect(failures).toEqual([lostUrl]);
  expect(errors).toEqual([]);
});

test("sépare lecture historique exacte, lecture actuelle partielle et droits révoqués", async ({
  page,
}, testInfo) => {
  const api = await installSourceApi(page, {
    attached: true,
    current: "partial",
  });
  await page.goto(`/sources/${sourceIds.reference}`);
  await page.getByText("Historique des lectures", { exact: true }).click();
  await page
    .getByRole("link", { name: "Lecture n°1 · [FICTIF] Durée des crédits" })
    .click();
  await expect(page).toHaveURL(`/source-observations/${sourceIds.original}`);
  await expect(page.getByRole("article")).toContainText(
    "Les crédits acquis restent disponibles",
  );
  await expect(page.getByRole("article")).not.toContainText(
    "politique de crédits modifiée",
  );
  await page.getByRole("link", { name: "Voir la lecture actuelle" }).click();
  await expect(page).toHaveURL(`/source-observations/${sourceIds.current}`);
  await expect(page.getByRole("article")).toContainText(
    "politique de crédits modifiée",
  );
  await expect(
    page.getByRole("region", { name: "Couverture de cette lecture" }),
  ).toContainText("Lecture partielle");
  await page.setViewportSize({ width: 390, height: 844 });
  await checkLayout(
    page,
    `/tmp/t17-partial-mobile-${testInfo.project.name}.png`,
  );
  api.denyExact();
  await page
    .getByRole("button", { name: "Recharger l’état enregistré" })
    .click();
  await expect(page.getByRole("alert")).toContainText("Accès refusé");
  await expect(page.getByText(/politique de crédits modifiée/)).toHaveCount(0);
  expect(api.requests.filter((r) => r.method() === "POST")).toHaveLength(0);
});

test("montre une source indisponible sans ressusciter son ancien texte et respecte le lecteur", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  const api = await installSourceApi(page, {
    attached: true,
    current: "unavailable",
    role: "viewer",
  });
  await page.goto(`/projects/${ids.project}/sources`);
  await expect(page.getByText(/Consultation uniquement/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Lire et ajouter la source" }),
  ).toHaveCount(0);
  await page.getByRole("link", { name: "[FICTIF] Durée des crédits" }).click();
  await expect(page.getByRole("article")).toContainText(
    "Source absente ou inaccessible",
  );
  await expect(page.getByRole("article")).not.toContainText(
    "Les crédits acquis restent disponibles",
  );
  for (const name of [
    "Vérifier dans l’outil",
    "Retirer du contexte",
    "Changer la connexion",
  ])
    await expect(page.getByRole("button", { name, exact: true })).toHaveCount(
      0,
    );
  await page.setViewportSize({ width: 390, height: 844 });
  await checkLayout(
    page,
    `/tmp/t17-unavailable-mobile-${testInfo.project.name}.png`,
  );
  expect(api.requests.filter((r) => r.method() === "POST")).toHaveLength(0);
  expect(errors).toEqual([]);
});

test("une observation de publication garde son origine sans commandes de rattachement", async ({
  page,
}) => {
  const api = await installSourceApi(page);
  await page.goto(`/publication-observations/${sourceIds.publication}`);
  await expect(page.getByRole("article")).toContainText(
    "Créée depuis un livrable",
  );
  await expect(
    page.getByRole("button", { name: "Retirer du contexte", exact: true }),
  ).toHaveCount(0);
  expect(api.requests.filter((r) => r.method() === "POST")).toHaveLength(0);
});

test("ouvre une source au clavier et conserve un titre long dans la largeur mobile", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await installSourceApi(page, { attached: true, longTitle: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`/sources/${sourceIds.reference}`);
  await expect(page.getByRole("heading", { level: 1 })).toBeFocused();
  await checkLayout(page, `/tmp/t17-long-title-${testInfo.project.name}.png`);
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Ouvrir dans Linear" }),
  ).toBeFocused();
  expect(errors).toEqual([]);
});
