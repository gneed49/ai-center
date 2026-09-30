import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { installMockApi, ids } from "./mock-api";
import { fixtureArtifact } from "../src/test-fixtures/artifacts";
import {
  fixtureTicketCoverage,
  fixtureTicketPreview,
  fixtureTicketResult,
  fixtureTicketTarget,
  fixtureTicketTools,
  fixtureTicketVersion,
} from "../src/test-fixtures/ticket-publications";
import type { Publication } from "../src/api/work-tool-types";

test("sélectionne des tickets distincts puis retrouve leurs reçus après réponse perdue et nouvelle version", async ({
  page,
}) => {
  const errors: string[] = [];
  const failedRequests: string[] = [];
  let deliberatelyAbortedUrl: string | null = null;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("requestfailed", (request) => failedRequests.push(request.url()));
  page.on("console", (message) => {
    if (message.type() !== "error") return;
    // Firefox reports an intentionally aborted cross-origin request as CORS.
    // Exempt only the injected failure; all other CORS/network errors still fail.
    const expectedInjectedFailure =
      deliberatelyAbortedUrl !== null &&
      ((message.location().url === deliberatelyAbortedUrl &&
        message.text() === "Failed to load resource: net::ERR_FAILED") ||
        message.text() ===
          `[JavaScript Error: "Cross-Origin Request Blocked: The Same Origin Policy disallows reading the remote resource at ${deliberatelyAbortedUrl}. (Reason: CORS request did not succeed). Status code: (null)."]`);
    if (!expectedInjectedFailure) errors.push(message.text());
  });
  await installMockApi(page, { companySuite: true });
  let calls = 0;
  let jobs: Publication[] = [];
  let source = fixtureTicketVersion;
  const saved = fixtureTicketVersion;
  await page.route("**/api/**", async (route) => {
    const req = route.request(),
      url = new URL(req.url()),
      path = url.pathname;
    if (path === "/api/artifacts/fixture-artifact")
      return route.fulfill({
        json: {
          ...fixtureArtifact,
          artifact: {
            ...fixtureArtifact.artifact,
            project_id: ids.project,
            artifact_type: "product_tickets",
            current_version_id: source.public_id,
            status: source.status,
            version: source.version,
          },
          current_version: source,
        },
      });
    if (path === "/api/artifacts/fixture-artifact/versions")
      return route.fulfill({
        json: {
          items: [source, saved],
          total: source.public_id === saved.public_id ? 1 : 2,
          limit: 25,
          offset: 0,
        },
      });
    if (path === `/api/artifacts/fixture-artifact/versions/${saved.public_id}`)
      return route.fulfill({ json: saved });
    if (path === "/api/work-tools")
      return route.fulfill({ json: fixtureTicketTools });
    if (path.endsWith("/artifact-destinations"))
      return route.fulfill({
        json: {
          items: [
            {
              artifact_type: "product_tickets",
              provider: "linear",
              target_id: fixtureTicketTarget,
              label: "[FICTIF] Équipe produit",
              origin: "company",
              revision: 1,
              project_id: null,
            },
          ],
        },
      });
    if (path === "/api/artifacts/fixture-artifact/ticket-publications/preview")
      return route.fulfill({ json: fixtureTicketPreview(req.postDataJSON()) });
    if (
      path === "/api/artifacts/fixture-artifact/ticket-publications" &&
      req.method() === "POST"
    ) {
      calls++;
      const input = req.postDataJSON();
      expect(input.ticket_indexes).toEqual([1, 29]);
      jobs = input.ticket_indexes.map((index: number, i: number) => ({
        ...fixtureTicketResult.publications[i],
        source_ticket_index: index,
        title: `[FICTIF] Travail ${index + 1}`,
      }));
      source = { ...saved, public_id: "fixture-version-3", version: 3 };
      deliberatelyAbortedUrl = req.url();
      return route.abort("failed");
    }
    if (path === "/api/artifacts/fixture-artifact/ticket-publications")
      return route.fulfill({
        json: {
          ...fixtureTicketCoverage,
          version_id: source.public_id,
          source_version_number: source.version,
        },
      });
    if (path.includes("/ticket-publication-commands/"))
      return route.fulfill({
        json: {
          status: jobs.length ? "completed" : "processing",
          can_retry: false,
          result: jobs.length
            ? {
                ...fixtureTicketResult,
                created_count: 2,
                publications: jobs,
              }
            : null,
        },
      });
    if (path.startsWith("/api/publications/fixture-ticket-publication-"))
      return route.fulfill({
        json: {
          publication: jobs.find((job) => path.endsWith(job.public_id)),
          observations: [],
        },
      });
    return route.fallback();
  });
  await page.goto("/artifacts/fixture-artifact");
  await expect(
    page.getByRole("heading", {
      name: fixtureTicketVersion.title,
      exact: true,
    }),
  ).toBeVisible();
  await expect(page).toHaveTitle(/AI Center/);
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  await page.screenshot({
    path: "/tmp/ai-center-ticket-document-desktop.png",
    fullPage: false,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: "/tmp/ai-center-ticket-document-mobile.png",
    fullPage: false,
  });
  await page.goto("/artifacts/fixture-artifact?ticket=29");
  const last = page.getByRole("article", {
    name: "Ticket 30 · version 2",
    exact: true,
  });
  await expect(last).toBeFocused();
  await expect(
    last.getByRole("heading", { name: "[FICTIF] Travail 30" }),
  ).toBeVisible();
  await page
    .getByRole("checkbox", {
      name: "Sélectionner Ticket 2 — [FICTIF] Travail 2",
      exact: true,
    })
    .check();
  await page
    .getByRole("checkbox", {
      name: "Sélectionner Ticket 30 — [FICTIF] Travail 30",
      exact: true,
    })
    .check();
  await page.getByRole("button", { name: "Préparer les 2 tickets" }).click();
  const preview = page.getByRole("region", {
    name: "Confirmation des tickets",
  });
  await expect(preview).toBeVisible();
  expect(calls).toBe(0);
  await preview
    .getByRole("button", {
      name: "Confirmer la création de 2 tickets dans Linear",
    })
    .click();
  await expect(
    page.getByText("La demande n’a pas pu être confirmée"),
  ).toBeVisible();
  await page.goto("/artifacts/fixture-artifact");
  const recovery = page.getByRole("region", {
    name: "Suivi des tickets demandés",
  });
  await expect(recovery).toContainText(
    "2 nouvelle(s) demande(s) enregistrée(s)",
  );
  await expect(recovery).toContainText(
    "Cette demande concerne une autre version",
  );
  await expect(
    recovery.getByText("Publication confirmée", { exact: true }),
  ).toBeVisible();
  await expect(
    recovery.getByText("Résultat à vérifier", { exact: true }),
  ).toBeVisible();
  expect(calls).toBe(1);
  await expect(
    recovery.getByRole("link", {
      name: "Ouvrir la version source · ticket 30",
    }),
  ).toHaveAttribute(
    "href",
    `/artifacts/fixture-artifact?version=${saved.public_id}&ticket=29`,
  );
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(axe.violations).toEqual([]);
  expect(failedRequests).toEqual([deliberatelyAbortedUrl]);
  expect(errors).toEqual([]);
  await recovery.scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "/tmp/ai-center-ticket-publication-mobile.png",
    fullPage: false,
  });
});
