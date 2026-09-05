import AxeBuilder from "@axe-core/playwright";
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { captureConsoleErrors } from "./mock-api";

test.use({
  baseURL: `http://127.0.0.1:${process.env.AI_CENTER_WEB_AUTH_E2E_PORT ?? "5184"}`,
});

const authStorageKey = "sb-127-auth-token";
const selectionKey = (actor: string) => `ai-center.workspace.v1:${actor}`;
const actorA = "10000000-0000-4000-8000-000000000001";
const actorB = "10000000-0000-4000-8000-000000000002";
const workspaceA = "20000000-0000-4000-8000-000000000001";
const workspaceB = "20000000-0000-4000-8000-000000000002";

function session(actor: string, suffix = "") {
  return {
    access_token: `fixture-${actor}${suffix}`,
    refresh_token: `fixture-refresh-${actor}`,
    token_type: "bearer",
    expires_in: 3600,
    expires_at: Math.floor(Date.now() / 1000) + 3600,
    user: {
      id: actor,
      aud: "authenticated",
      role: "authenticated",
      email: `${actor}@example.test`,
      app_metadata: {},
      user_metadata: {},
      created_at: "2026-01-01T00:00:00Z",
    },
  };
}

async function seedIdentity(context: BrowserContext) {
  await context.addInitScript(
    ({ key, selection, value, workspace }) => {
      if (!localStorage.getItem(key)) {
        localStorage.setItem(key, JSON.stringify(value));
        localStorage.setItem(selection, workspace);
      }
    },
    {
      key: authStorageKey,
      selection: selectionKey(actorA),
      value: session(actorA),
      workspace: workspaceA,
    },
  );
}

async function publishSession(
  page: Page,
  actor: string,
  event = "SIGNED_IN",
  suffix = "",
) {
  await page.evaluate(
    ({ key, value, event }) => {
      localStorage.setItem(key, JSON.stringify(value));
      // Exercise Supabase's cross-tab event transport without exposing an app test hook.
      const channel = new BroadcastChannel(key);
      channel.postMessage({ event, session: value });
      setTimeout(() => channel.close(), 100);
    },
    { key: authStorageKey, value: session(actor, suffix), event },
  );
}

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function installIdentityApi(
  context: BrowserContext,
  options: {
    delayedProjects?: ReturnType<typeof deferred>;
    delayedMutation?: ReturnType<typeof deferred>;
  } = {},
) {
  const requests: Array<{
    path: string;
    method: string;
    token?: string;
    workspace?: string;
  }> = [];
  await context.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (!path.startsWith("/api/")) return route.continue();
    const token = request.headers().authorization;
    const workspace = request.headers()["x-ai-center-workspace-id"];
    requests.push({ path, method: request.method(), token, workspace });
    const beta = token === `Bearer fixture-${actorB}`;
    if (path === "/api/workspaces")
      return route.fulfill({
        json: [
          {
            public_id: beta ? workspaceB : workspaceA,
            name: beta ? "Workspace Beta" : "Workspace Alpha",
            role: "owner",
          },
        ],
      });
    if (path === "/api/projects") {
      if (request.method() === "POST") await options.delayedMutation?.promise;
      else if (!beta) await options.delayedProjects?.promise;
      const project = {
        public_id: beta ? "project-beta" : "project-alpha",
        name: beta ? "Projet Beta" : "Projet Alpha confidentiel",
        objective: "Contexte du compte courant",
        status: "active",
        graph_version: 1,
        updated_at: "2026-09-05T00:00:00Z",
      };
      return route.fulfill({
        json: request.method() === "POST" ? project : [project],
      });
    }
    return route.fulfill({
      status: 404,
      json: { code: "not_found", message: "Ressource absente du contexte" },
    });
  });
  return requests;
}

test("change de compte sans réutiliser son cache ni son workspace", async ({
  page,
  context,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  await seedIdentity(context);
  const requests = await installIdentityApi(context);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Projet Alpha confidentiel" }),
  ).toBeVisible();
  await publishSession(page, actorB);
  await expect(
    page.getByRole("heading", { name: "Choisir le contexte partagé" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: /Workspace Beta/ }),
  ).toBeVisible();
  await expect(page.getByText("Projet Alpha confidentiel")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: /Workspace Alpha/ }),
  ).toHaveCount(0);
  const audit = await new AxeBuilder({ page }).analyze();
  expect(
    audit.violations.filter((item) =>
      ["critical", "serious"].includes(item.impact ?? ""),
    ),
  ).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath("identity-picker.png") });
  await page.getByRole("button", { name: /Workspace Beta/ }).click();
  await expect(
    page.getByRole("heading", { name: "Projet Beta" }),
  ).toBeVisible();
  expect(
    requests
      .filter((request) => request.path === "/api/projects")
      .every(
        (request) =>
          (request.token === `Bearer fixture-${actorA}` &&
            request.workspace === workspaceA) ||
          (request.token === `Bearer fixture-${actorB}` &&
            request.workspace === workspaceB),
      ),
  ).toBe(true);
  expect(
    await page.evaluate(() => localStorage.getItem("ai-center.access-token")),
  ).toBeNull();
  expect(errors).toEqual([]);
});

test("annule une lecture de l’ancien compte et ignore sa réponse tardive", async ({
  page,
  context,
}) => {
  const errors = captureConsoleErrors(page);
  const delayedProjects = deferred();
  await seedIdentity(context);
  const requests = await installIdentityApi(context, { delayedProjects });
  await page.goto("/");
  await expect
    .poll(() => requests.some((item) => item.path === "/api/projects"))
    .toBe(true);
  await publishSession(page, actorB);
  await expect(
    page.getByRole("button", { name: /Workspace Beta/ }),
  ).toBeVisible();
  delayedProjects.resolve();
  await page.getByRole("button", { name: /Workspace Beta/ }).click();
  await expect(
    page.getByRole("heading", { name: "Projet Beta" }),
  ).toBeVisible();
  await expect(page.getByText("Projet Alpha confidentiel")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("change simultanément les onglets et retire les drafts de l’ancien compte", async ({
  page,
  context,
}) => {
  const errors = captureConsoleErrors(page);
  await seedIdentity(context);
  await installIdentityApi(context);
  await page.goto("/");
  const other = await context.newPage();
  const otherErrors = captureConsoleErrors(other);
  await other.goto(`${new URL(page.url()).origin}/projects/new`);
  await other.getByLabel("Nom du projet").fill("Draft privé Alpha");
  await publishSession(page, actorB);
  for (const tab of [page, other]) {
    await expect(
      tab.getByRole("button", { name: /Workspace Beta/ }),
    ).toBeVisible();
    await expect(tab.getByText("Projet Alpha confidentiel")).toHaveCount(0);
  }
  await page.getByRole("button", { name: /Workspace Beta/ }).click();
  await expect(
    page.getByRole("heading", { name: "Projet Beta" }),
  ).toBeVisible();
  await expect(other.getByLabel("Nom du projet")).toHaveValue("");
  await expect(other.getByLabel("Objectif initial")).toHaveValue("");
  expect(errors).toEqual([]);
  expect(otherErrors).toEqual([]);
});

test("préserve le draft et la sélection pendant le renouvellement du même compte", async ({
  page,
  context,
}) => {
  const errors = captureConsoleErrors(page);
  await seedIdentity(context);
  await installIdentityApi(context);
  await page.goto("/projects/new");
  await page.getByLabel("Nom du projet").fill("Draft Alpha conservé");
  await page.getByLabel("Objectif initial").fill("Objectif conservé");
  await publishSession(page, actorA, "TOKEN_REFRESHED", "-refreshed");
  await expect(page.getByLabel("Nom du projet")).toHaveValue(
    "Draft Alpha conservé",
  );
  await expect(page.getByLabel("Objectif initial")).toHaveValue(
    "Objectif conservé",
  );
  await expect(
    page.getByRole("heading", { name: "Choisir le contexte partagé" }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("une création terminée sous l’ancien compte ne redirige pas le nouveau", async ({
  page,
  context,
}) => {
  const errors = captureConsoleErrors(page);
  const delayedMutation = deferred();
  await seedIdentity(context);
  const requests = await installIdentityApi(context, { delayedMutation });
  await page.goto("/projects/new");
  await page.getByLabel("Nom du projet").fill("Draft privé Alpha");
  await page
    .getByLabel("Objectif initial")
    .fill("Ne doit pas apparaître sous Beta");
  await page.getByRole("button", { name: "Créer les scopes" }).click();
  await expect
    .poll(() => requests.some((item) => item.method === "POST"))
    .toBe(true);
  await publishSession(page, actorB);
  await expect(
    page.getByRole("button", { name: /Workspace Beta/ }),
  ).toBeVisible();
  delayedMutation.resolve();
  await page.getByRole("button", { name: /Workspace Beta/ }).click();
  await expect(
    page.getByRole("heading", { name: "Projet Beta" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await expect(
    page.getByText("Le contexte de connexion a changé."),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("affiche l’échec du lien magique même après la restauration anonyme", async ({
  page,
}) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      "sb-127-auth-token-code-verifier",
      JSON.stringify("local-pkce-fixture"),
    );
  });
  await page.route("http://127.0.0.1:54329/auth/v1/token**", async (route) => {
    await route.fulfill({
      status: 400,
      json: {
        error: "invalid_grant",
        error_description: "Lien de connexion expiré dans la fixture",
      },
    });
  });
  await page.goto("/auth/callback?code=expired-fixture");
  await expect(page.getByRole("alert")).toContainText(
    "Ce lien n’a pas pu être vérifié.",
  );
  await expect(page.getByText("Vérification du lien sécurisé…")).toHaveCount(0);
  await page.getByRole("link", { name: "Revenir à la connexion" }).click();
  await expect(page.getByLabel("Adresse e-mail")).toBeVisible();
});
