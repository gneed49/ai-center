import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type {
  SourceCommandResult,
  SourceObservationDetail,
  ToolSourceList,
} from "../src/api/tool-source-types";

// [FICTIF] Real application HTTP/auth/RLS/PostgreSQL. The harness injects only
// loopback provider responses; this does not prove any real tool account access.
function fixtureId(name: string) {
  const id = process.env[name];
  if (
    !id ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id)
  )
    throw new Error(`The guarded source harness must provide ${name}.`);
  return id;
}
const project = fixtureId("E2E_SOURCE_PROJECT");
const api = process.env.E2E_API_URL!;
const providerOrigin = process.env.E2E_SOURCE_PROVIDER!;
const sourcePage = fixtureId("E2E_SOURCE_PAGE");
const connections = {
  linear: fixtureId("E2E_LINEAR_CONNECTION_ID"),
  notion: fixtureId("E2E_NOTION_CONNECTION_ID"),
};
async function attach(
  page: Page,
  provider: "linear" | "notion",
  source: string,
) {
  const form = page.getByRole("region", {
    name: "Ajouter une source",
    exact: true,
  });
  await form
    .getByRole("combobox", { name: "Outil", exact: true })
    .selectOption(provider);
  await form
    .getByLabel("Connexion autorisée")
    .selectOption(connections[provider]);
  await form.getByLabel("Lien de la page ou du ticket").fill(source);
  await expect(
    form.getByRole("button", { name: "Lire et ajouter la source" }),
  ).toBeDisabled();
  await form.getByRole("checkbox").check();
  await form.getByRole("button", { name: "Lire et ajouter la source" }).click();
  await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
}
async function visualCheck(page: Page, path: string) {
  // Start captures at the page origin. A viewport change can retain scroll,
  // causing a sticky header to obscure breadcrumbs in a full-page screenshot.
  await page.evaluate(() => window.scrollTo(0, 0));
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
  await expect(page.getByRole("heading", { level: 1 })).toBeInViewport();
  await expect(
    page.getByRole("navigation", { name: "Contexte de la lecture" }),
  ).toBeInViewport();
  const placement = await page.evaluate(() => ({
    scroll: window.scrollY,
    titleTop: document.querySelector("h1")!.getBoundingClientRect().top,
    contextTop: document
      .querySelector('nav[aria-label="Contexte de la lecture"]')!
      .getBoundingClientRect().top,
    headerBottom: document
      .querySelector("header.mobile-safe-header")!
      .getBoundingClientRect().bottom,
  }));
  expect(placement.scroll).toBe(0);
  expect(placement.titleTop).toBeGreaterThanOrEqual(placement.headerBottom);
  expect(placement.contextTop).toBeGreaterThanOrEqual(placement.headerBottom);
  const audit = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(
    audit.violations.map((v) => ({
      id: v.id,
      targets: v.nodes.map((n) => n.target),
    })),
  ).toEqual([]);
  await page.screenshot({ path, fullPage: true });
}

test("relie Linear et Notion existants, récupère le reçu perdu puis conserve deux lectures exactes", async ({
  page,
  request,
}) => {
  const attachUrl = `${api}/api/projects/${project}/tool-sources`;
  const errors: string[] = [],
    failedRequests: string[] = [];
  let lost = false,
    calls = 0;
  let first: SourceCommandResult | undefined;
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("requestfailed", (req) => failedRequests.push(req.url()));
  page.on("console", (message) => {
    if (message.type() !== "error") return;
    const expected =
      lost &&
      message.location().url === attachUrl &&
      message.text() === "Failed to load resource: net::ERR_FAILED";
    if (!expected) errors.push(message.text());
  });
  await page.route(attachUrl, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    calls++;
    // Execute the actual mutation once. Lose only the response delivered to UI.
    const response = await route.fetch({ maxRetries: 0 });
    expect(response.status()).toBe(200);
    if (!lost) {
      first = (await response.json()) as SourceCommandResult;
      lost = true;
      await route.abort("failed");
    } else await route.fulfill({ response });
  });
  await page.goto(`/projects/${project}/sources`);
  await expect(
    page.getByRole("heading", { name: "Sources du projet" }),
  ).toBeVisible();
  await attach(page, "linear", "PROD-42");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(page.getByText(/Source ajoutée ·/)).toBeVisible();
  expect(calls).toBe(1);
  await page
    .getByRole("link", { name: "Ouvrir la lecture de cette demande" })
    .click();
  await expect(page).toHaveURL(
    `/source-observations/${first!.observation.public_id}`,
  );
  await expect(page.getByRole("article")).toContainText(
    "Les crédits restent disponibles sans expiration.",
  );
  await expect(page.getByRole("article")).toContainText("[FICTIF] À faire");
  await visualCheck(page, "/tmp/t17-real-linear-desktop.png");

  await page.goto(`/projects/${project}/sources`);
  await page
    .getByRole("button", { name: "Préparer une nouvelle demande" })
    .click();
  await attach(page, "notion", sourcePage);
  expect(calls).toBe(2);
  const listResponse = await request.get(`${attachUrl}?limit=25&status=active`);
  expect(listResponse.status()).toBe(200);
  const list = (await listResponse.json()) as ToolSourceList;
  expect(list.items).toHaveLength(2);
  const notion = list.items.find(
    (item) => item.reference.provider === "notion",
  )!;
  expect(notion).toBeDefined();
  const originalId = notion.observation.public_id;
  await page
    .getByRole("link", { name: "Voir l’état actuel de la source" })
    .click();
  await expect(page).toHaveURL(`/sources/${notion.reference.public_id}`);
  await expect(page.getByRole("article")).toContainText("Version distante 1.");
  const advanced = await request.post(`${providerOrigin}fixture/advance`);
  expect(advanced.status()).toBe(204);
  const refreshed = page.waitForResponse(
    (r) =>
      r.url() ===
        `${api}/api/tool-sources/${notion.reference.public_id}/refresh` &&
      r.request().method() === "POST",
  );
  await page
    .getByRole("button", { name: "Vérifier dans l’outil", exact: true })
    .click();
  const refreshResponse = await refreshed;
  expect(refreshResponse.status()).toBe(200);
  const current = (await refreshResponse.json()) as SourceCommandResult;
  expect(current.effect).toBe("changed");
  expect(current.observation.version).toBe(2);
  expect(current.observation.public_id).not.toBe(originalId);
  await expect(page.getByRole("article")).toContainText("Version distante 2.");
  await expect(
    page.getByRole("region", { name: "Couverture de cette lecture" }),
  ).toContainText("Lecture partielle");
  await page.getByText("Historique des lectures", { exact: true }).click();
  const originalLink = page.getByRole("link", {
    name: "Lecture n°1 · [FICTIF] Spécification existante",
  });
  await expect(originalLink).toHaveAttribute(
    "href",
    `/source-observations/${originalId}`,
  );
  await originalLink.click();
  await expect(page.getByRole("article")).toContainText("Version distante 1.");
  await expect(page.getByRole("article")).not.toContainText(
    "Version distante 2.",
  );
  await expect(
    page.getByText("Vous consultez une lecture historique."),
  ).toBeVisible();
  await page.getByRole("link", { name: "Voir la lecture actuelle" }).click();
  await expect(page).toHaveURL(
    `/source-observations/${current.observation.public_id}`,
  );
  await expect(page.getByRole("article")).toContainText("Version distante 2.");
  await page.setViewportSize({ width: 390, height: 844 });
  await visualCheck(page, "/tmp/t17-real-notion-mobile.png");
  // A second API read proves the historical object remained immutable.
  const oldResponse = await request.get(
    `${api}/api/tool-source-observations/${originalId}`,
  );
  expect(oldResponse.status()).toBe(200);
  const old = (await oldResponse.json()) as SourceObservationDetail;
  expect(old.body_markdown).toContain("Version distante 1.");
  expect(old.observation.freshness.is_current).toBe(false);
  expect(old.observation.freshness.current_observation_id).toBe(
    current.observation.public_id,
  );
  expect(calls).toBe(2);
  expect(failedRequests).toEqual([attachUrl]);
  expect(errors).toEqual([]);
});
