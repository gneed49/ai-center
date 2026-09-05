import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { captureConsoleErrors, ids, installMockApi } from "./mock-api";

test("conserve le message hors ligne et ne l’envoie qu’une fois après reconnexion", async ({
  page,
  context,
}) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page);
  await page.goto(`/projects/${ids.project}/sessions/${ids.techSession}`);
  const composer = page.getByLabel("Message à l’agent tech");
  await composer.fill("Saisie conservée malgré la coupure");
  await context.setOffline(true);
  await expect(
    page.getByText(/Hors connexion · vos saisies sont conservées/),
  ).toBeVisible();
  await page.getByRole("button", { name: "Envoyer" }).click();
  await expect(composer).toHaveValue("Saisie conservée malgré la coupure");
  expect(state.messageMutations).toBe(0);
  await context.setOffline(false);
  await expect(composer).toHaveValue("");
  await expect(
    page.getByText(/Hors connexion · vos saisies sont conservées/),
  ).toHaveCount(0);
  expect(state.messageProviderAttempts).toBe(1);
  expect(state.messageMutations).toBe(1);
  expect(errors).toEqual([]);
});

test("conserve une résolution en conflit puis exige des sources actualisées", async ({
  page,
}, testInfo) => {
  const errors = captureConsoleErrors(page);
  const state = await installMockApi(page, {
    resolutionConflictOnce: true,
    resolutionDelayMs: 250,
  });
  await page.goto(`/projects/${ids.project}/insights/${ids.insight}`);
  const justification = page.getByLabel("Justification de la décision");
  await justification.fill("Révision explicite après relecture du contexte");
  await page.getByRole("button", { name: "Réviser pour résoudre" }).click();
  const draft = page.getByLabel("Nouvelle vérité du projet");
  await draft.fill("Les données sont conservées 90 jours puis supprimées.");
  await page.getByRole("button", { name: "Réviser et résoudre" }).click();
  await expect(draft).toBeDisabled();
  await expect(justification).toBeDisabled();
  await expect(
    page.getByText(/Le contexte a changé. Rechargez les sources/),
  ).toBeVisible();
  await expect(draft).toHaveValue(
    "Les données sont conservées 90 jours puis supprimées.",
  );
  await expect(
    page.getByRole("button", { name: "Réviser et résoudre" }),
  ).toBeDisabled();
  await expect(page.getByRole("button", { name: "Réessayer" })).toHaveCount(0);
  await page.screenshot({
    path: testInfo.outputPath("resolution-conflict.png"),
  });
  await page.getByRole("button", { name: "Recharger les sources" }).click();
  await expect(
    page.getByText(/Contexte actualisé. Vérifiez les sources/),
  ).toBeVisible();
  await expect(draft).toHaveValue(
    "Les données sont conservées 90 jours puis supprimées.",
  );
  await page.getByRole("button", { name: "Réviser et résoudre" }).click();
  await expect(page.getByText("resolved")).toBeVisible();
  const attempts = state.requests.filter((request) =>
    request.url().endsWith(`/insights/${ids.insight}/resolve`),
  );
  expect(attempts).toHaveLength(2);
  expect(attempts[0].postDataJSON().expected_graph_version).toBe(4);
  expect(attempts[1].postDataJSON().expected_graph_version).toBe(5);
  expect(attempts[1].postDataJSON().mutations).toEqual(
    attempts[0].postDataJSON().mutations,
  );
  expect(attempts[1].headers()["idempotency-key"]).not.toBe(
    attempts[0].headers()["idempotency-key"],
  );
  expect(errors.every((message) => message.includes("409"))).toBe(true);
});

const routes = [
  {
    name: "Center",
    page: "/",
    resource: "/api/projects",
    ready: "Un seul endroit pour garder le contexte vivant.",
  },
  {
    name: "Projet",
    page: `/projects/${ids.project}`,
    resource: `/api/projects/${ids.project}/snapshot`,
    ready: "AI Center",
  },
  {
    name: "Session",
    page: `/projects/${ids.project}/sessions/${ids.techSession}`,
    resource: `/api/projects/${ids.project}/sessions/${ids.techSession}`,
    ready: "Message à l’agent tech",
    label: true,
  },
  {
    name: "Livrables",
    page: `/projects/${ids.project}/deliverables`,
    resource: `/api/projects/${ids.project}/snapshot`,
    ready: "Livrables et couverture",
  },
  {
    name: "Détail livrable",
    page: `/projects/${ids.project}/deliverables/${ids.deliverable}`,
    resource: `/api/projects/${ids.project}/snapshot`,
    ready: "Plan de preuve externe",
  },
  {
    name: "Handoff",
    page: `/projects/${ids.project}/handoff`,
    resource: `/api/projects/${ids.project}/handoffs/latest`,
    ready: "Handoff Produit → Tech",
  },
  {
    name: "Inbox",
    page: "/insights",
    resource: "/api/insights",
    ready: "Les signaux qui méritent une décision.",
  },
  {
    name: "Signal",
    page: `/projects/${ids.project}/insights/${ids.insight}`,
    resource: `/api/projects/${ids.project}/insights/${ids.insight}`,
    ready: "Rétention contradictoire",
  },
  {
    name: "Historique",
    page: `/projects/${ids.project}/history`,
    resource: `/api/projects/${ids.project}/history`,
    ready: "Historique du projet",
  },
];

for (const scenario of routes) {
  test(`${scenario.name} : chargement, erreur explicite et reprise de la route`, async ({
    page,
  }) => {
    const errors = captureConsoleErrors(page);
    await installMockApi(page, { externalProof: true });
    let release!: () => void;
    const loading = new Promise<void>((resolve) => {
      release = resolve;
    });
    let failed = true;
    await page.route(
      `http://127.0.0.1:4317${scenario.resource}`,
      async (route) => {
        if (!failed) return route.fallback();
        await loading;
        return route.fulfill({
          status: 503,
          json: {
            code: "provider_unavailable",
            message: "Service temporairement indisponible pour cette route.",
          },
        });
      },
    );
    await page.goto(scenario.page);
    await expect(page.locator('[aria-busy="true"]').first()).toBeVisible();
    release();
    await expect(
      page.getByText("Service temporairement indisponible pour cette route."),
    ).toBeVisible();
    const audit = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(
      audit.violations.filter((item) =>
        ["critical", "serious"].includes(item.impact ?? ""),
      ),
    ).toEqual([]);
    failed = false;
    await page.getByRole("button", { name: "Réessayer" }).click();
    await expect(
      page.getByText("Service temporairement indisponible pour cette route."),
    ).toHaveCount(0);
    await expect(
      scenario.label
        ? page.getByLabel(scenario.ready)
        : page.getByRole("heading", { name: scenario.ready, exact: true }),
    ).toBeVisible();
    expect(errors.every((message) => message.includes("503"))).toBe(true);
  });
}

test("un livrable étranger reste hors contexte et l’historique vide est explicite", async ({
  page,
}) => {
  const errors = captureConsoleErrors(page);
  await installMockApi(page, { externalProof: true });
  await page.goto(`/projects/${ids.project}/deliverables/foreign-deliverable`);
  await expect(
    page.getByRole("heading", { name: "Livrable introuvable dans ce projet" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Plan de preuve externe" }),
  ).toHaveCount(0);
  await page.goto(`/projects/${ids.project}/history`);
  await expect(
    page.getByText("Aucun événement", { exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
