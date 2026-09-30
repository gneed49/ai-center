import AxeBuilder from "@axe-core/playwright";
import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from "@playwright/test";
import type { ArtifactDetail, ArtifactType } from "../src/api/artifact-types";
import type { TicketPublicationResult } from "../src/api/ticket-publication-types";
import type { Publication } from "../src/api/work-tool-types";
import {
  draftMarkdown,
  typedDraft,
} from "../src/components/artifacts/typed-draft";

// Real scoped HTTP API and disposable PostgreSQL. Only external tool HTTP endpoints
// and the agent are deterministic fixtures. The model generates ONE ticket; this
// test explicitly adds authored [FICTIF] entries through the revision editor,
// before human review/validation/publication in the browser. It does not prove a live
// model generates N tickets, and does not replace the API or publication receipts.
const api = process.env.E2E_API_URL ?? process.env.AI_CENTER_REAL_E2E_API_URL!;
async function post<T>(
  request: APIRequestContext,
  path: string,
  data: unknown,
): Promise<T> {
  const response = await request.post(`${api}${path}`, {
    headers: { "Idempotency-Key": crypto.randomUUID() },
    data,
  });
  expect(response.status(), path).toBe(200);
  return response.json() as Promise<T>;
}
async function get<T>(request: APIRequestContext, path: string): Promise<T> {
  const response = await request.get(`${api}${path}`);
  expect(response.status(), path).toBe(200);
  return response.json() as Promise<T>;
}
async function generate(
  page: Page,
  project: string,
  session: string,
  type: ArtifactType,
) {
  await page.goto(
    `/projects/${project}/artifacts?create=1&session=${session}&draft_type=${type}`,
  );
  await expect(
    page.getByLabel("Type de livrable", { exact: true }),
  ).toHaveValue(type);
  await expect(page.getByLabel("Conversation de départ")).toHaveValue(session);
  await page
    .getByLabel("Ce que le document doit préparer")
    .fill("[FICTIF] Préparer les accès sur invitation et leur vérification.");
  const response = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      r.url().endsWith("/artifacts/generate"),
  );
  await page
    .getByRole("button", { name: "Générer le brouillon avec l’agent" })
    .click();
  const generated = await response;
  expect(generated.status()).toBe(200);
  const detail: ArtifactDetail = await generated.json();
  expect(
    typedDraft(detail.current_version.structured_content)?.tickets,
  ).toHaveLength(1);
  await expect(page).toHaveURL(
    new RegExp(`/artifacts/${detail.artifact.public_id}$`),
  );
  return detail;
}
async function authoredTickets(
  page: Page,
  request: APIRequestContext,
  generated: ArtifactDetail,
  count: number,
  label: string,
) {
  await page.goto(`/artifacts/${generated.artifact.public_id}`);
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page
    .getByLabel("Titre du livrable")
    .fill(`[FICTIF] ${label} · ${count} tickets relus`);
  for (let i = 0; i < count; i++) {
    if (i > 0) {
      await page.getByRole("button", { name: "Ajouter un ticket" }).click();
      await expect(
        page.getByRole("textbox", {
          name: `Titre du ticket ${i + 1}`,
          exact: true,
        }),
      ).toBeFocused();
    }
    await page
      .getByRole("textbox", { name: `Titre du ticket ${i + 1}`, exact: true })
      .fill(`[FICTIF] ${label} ${i + 1}`);
    await page
      .getByRole("textbox", {
        name: `Description du ticket ${i + 1}`,
        exact: true,
      })
      .fill(
        `[FICTIF] Réaliser le travail ${i + 1} pour le parcours d’invitation.`,
      );
    await page
      .getByRole("textbox", {
        name: `Critères d’acceptation du ticket ${i + 1} · un par ligne`,
        exact: true,
      })
      .fill(`[FICTIF] Le résultat ${i + 1} est vérifié contre les sources.`);
  }
  if (count === 3) {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        ),
    );
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    const sources = page.getByRole("region", {
      name: "Liste des sources de cette version",
    });
    await page
      .getByRole("textbox", { name: "Points à clarifier · un par ligne" })
      .focus();
    await page.keyboard.press("Tab");
    await expect(sources).toBeFocused();
    expect(
      await sources.evaluate(
        (element) => element.scrollHeight > element.clientHeight,
      ),
    ).toBe(true);
    await sources.press("End");
    await expect
      .poll(() => sources.evaluate((element) => element.scrollTop))
      .toBeGreaterThan(0);
    const axe = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(axe.violations).toEqual([]);
    await page.screenshot({
      path: "/tmp/t19-ticket-editor-mobile.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 1440, height: 900 });
  }
  const saved = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response
        .url()
        .endsWith(`/artifacts/${generated.artifact.public_id}/draft`),
  );
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  const response = await saved;
  expect(response.status()).toBe(200);
  const result = (await response.json()) as ArtifactDetail;
  const draft = typedDraft(result.current_version.structured_content)!;
  expect(draft.tickets).toHaveLength(count);
  expect(draft.tickets[0].source_ids).toEqual(
    typedDraft(generated.current_version.structured_content)!.tickets[0]
      .source_ids,
  );
  for (const ticket of draft.tickets.slice(1))
    expect(ticket.source_ids).toEqual([]);
  expect(draft.sections).toEqual(
    typedDraft(generated.current_version.structured_content)!.sections,
  );
  expect(result.current_version.body_markdown).toBe(draftMarkdown(draft));
  expect(result.current_version.sources).toEqual(
    generated.current_version.sources,
  );
  expect(result.current_version.status).toBe("draft");
  const original = await get<ArtifactDetail["current_version"]>(
    request,
    `/api/artifacts/${generated.artifact.public_id}/versions/${generated.current_version.public_id}`,
  );
  expect(original).toEqual(generated.current_version);
  return result;
}
async function reviewAndValidate(
  page: Page,
  revised: ArtifactDetail,
  count: number,
) {
  await page.goto(`/artifacts/${revised.artifact.public_id}`);
  await expect(
    page.getByRole("heading", {
      name: revised.current_version.title,
      exact: true,
    }),
  ).toBeVisible();
  for (let i = 0; i < count; i++)
    await expect(
      page.getByRole("article", {
        name: `Ticket ${i + 1} · version 2`,
        exact: true,
      }),
    ).toContainText(`[FICTIF] Le résultat ${i + 1}`);
  const response = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      r.url().endsWith(`/artifacts/${revised.artifact.public_id}/validate`),
  );
  await page.getByRole("button", { name: "Valider cette version" }).click();
  const validated = await response;
  expect(validated.status()).toBe(200);
  await expect(page.getByText(/Version 3 · Validée/)).toBeVisible();
  return validated.json() as Promise<ArtifactDetail>;
}
async function publishAndRecover(
  page: Page,
  request: APIRequestContext,
  document: ArtifactDetail,
  count: number,
  provider: "linear" | "github",
  loseResponse: boolean,
) {
  let calls = 0;
  let admitted: TicketPublicationResult | undefined;
  const endpoint = `**/api/artifacts/${document.artifact.public_id}/ticket-publications`;
  await page.route(endpoint, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    calls++;
    const response = await route.fetch();
    expect(response.status()).toBe(200);
    admitted = await response.json();
    if (loseResponse) await route.abort("failed");
    else await route.fulfill({ response });
  });
  for (let i = 0; i < count; i++)
    await page
      .getByRole("checkbox", {
        name: `Sélectionner Ticket ${i + 1} — ${typedDraft(document.current_version.structured_content)!.tickets[i].title}`,
        exact: true,
      })
      .check();
  await page
    .getByRole("button", { name: `Préparer les ${count} tickets` })
    .click();
  const preview = page.getByRole("region", {
    name: "Confirmation des tickets",
  });
  await expect(preview).toBeVisible();
  expect(calls).toBe(0);
  await expect(preview).toContainText(
    `${count} nouveau(x) ticket(s) seront créés`,
  );
  await preview
    .getByRole("button", {
      name: `Confirmer la création de ${count} tickets dans ${provider === "linear" ? "Linear" : "GitHub"}`,
    })
    .click();
  if (loseResponse)
    await expect(
      page.getByText("La demande n’a pas pu être confirmée"),
    ).toBeVisible();
  await expect.poll(() => admitted?.publications.length).toBe(count);
  const originalIds = admitted!.publications.map((job) => job.public_id).sort();
  expect(admitted!.created_count).toBe(count);
  expect(admitted!.existing_count).toBe(0);
  await page.reload();
  const recovery = page.getByRole("region", {
    name: "Suivi des tickets demandés",
  });
  await expect(recovery).toContainText(
    `${count} nouvelle(s) demande(s) enregistrée(s)`,
  );
  await expect
    .poll(
      async () => {
        const list = await get<{ items: Publication[] }>(
          request,
          `/api/artifacts/${document.artifact.public_id}/publications?limit=30&offset=0`,
        );
        return list.items.filter((job) => job.status === "succeeded").length;
      },
      { timeout: 30_000 },
    )
    .toBe(count);
  const list = await get<{ items: Publication[] }>(
    request,
    `/api/artifacts/${document.artifact.public_id}/publications?limit=30&offset=0`,
  );
  expect(list.items.map((job) => job.public_id).sort()).toEqual(originalIds);
  expect(new Set(list.items.map((job) => job.external_id)).size).toBe(count);
  expect(list.items.map((job) => job.source_ticket_index).sort()).toEqual(
    Array.from({ length: count }, (_, i) => i),
  );
  expect(
    list.items.every(
      (job) =>
        job.version_id === document.current_version.public_id &&
        job.provider === provider &&
        job.attempt_count === 1,
    ),
  ).toBe(true);
  await expect(
    recovery.getByText("Publication confirmée", { exact: true }),
  ).toHaveCount(count);
  expect(calls).toBe(1);
  await page.unroute(endpoint);
  return list.items;
}

test("PM 2 tickets Linear puis lead 3 tickets GitHub conservent leurs versions et leurs reçus", async ({
  page,
  request,
}, testInfo) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const runtimeErrors: string[] = [];
  page.on("pageerror", (error) => runtimeErrors.push(error.message));
  await post(request, "/api/company/setup", {
    name: "[FICTIF] Équipe TP-011",
    description: "Recette locale avec faux outils HTTP.",
  });
  const project = await post<{ public_id: string }>(request, "/api/projects", {
    name: `[FICTIF] Tickets ${crypto.randomUUID().slice(0, 8)}`,
    objective:
      "[FICTIF] Partager des invitations traçables entre Produit et Tech.",
  });
  for (const provider of ["linear", "github"] as const) {
    const id =
      provider === "linear"
        ? process.env.E2E_LINEAR_CONNECTION_ID
        : process.env.E2E_CONNECTION_ID;
    if (!id)
      await post(request, "/api/work-tools/connections", {
        id: crypto.randomUUID(),
        provider,
        name: `[FICTIF] ${provider}`,
        expected_revision: 0,
        api_key: "synthetic-credential-no-real-account",
      });
    await post(
      request,
      `/api/projects/${project.public_id}/artifact-destinations`,
      {
        artifact_type:
          provider === "linear" ? "product_tickets" : "technical_tickets",
        provider,
        target_id:
          provider === "linear"
            ? (process.env.E2E_LINEAR_TARGET_ID ??
              "11111111-1111-4111-8111-111111111111")
            : (process.env.E2E_TARGET_ID ?? "fictif/repository"),
        label: `[FICTIF] Destination ${provider}`,
        expected_revision: 0,
      },
    );
  }
  const session = await post<{ session: { public_id: string } }>(
    request,
    `/api/projects/${project.public_id}/sessions`,
    { node_key: "product", title: "[FICTIF] Invitations Produit" },
  );
  await page.goto(
    `/projects/${project.public_id}/sessions/${session.session.public_id}`,
  );
  await page
    .getByLabel("Message à l’agent product")
    .fill(
      "[FICTIF] Les invitations accordent un accès lecteur, les décisions restent consultables sans expiration.",
    );
  await page.getByRole("button", { name: "Envoyer", exact: true }).click();
  const proposals = page.getByRole("button", {
    name: /^Sélectionner la proposition/,
  });
  await expect(proposals).toHaveCount(3);
  for (let i = 0; i < 3; i++) await proposals.first().click();
  await page.getByRole("button", { name: "Confirmer (3)" }).click();
  await expect(page.getByText("3 connaissance(s) confirmée(s)")).toBeVisible();
  const pm = await generate(
    page,
    project.public_id,
    session.session.public_id,
    "product_tickets",
  );
  const product = await reviewAndValidate(
    page,
    await authoredTickets(page, request, pm, 2, "Produit"),
    2,
  );
  const productJobs = await publishAndRecover(
    page,
    request,
    product,
    2,
    "linear",
    true,
  );
  await page.goto(`/projects/${project.public_id}/handoff`);
  await page
    .getByRole("button", { name: "Vérifier les éléments produit" })
    .click();
  const transfer = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      r.url().endsWith(`/projects/${project.public_id}/handoffs`),
  );
  await page
    .getByRole("button", { name: "Transmettre et ouvrir la session Tech" })
    .click();
  const transferred = await transfer;
  expect(transferred.status()).toBe(200);
  const handoff: { target_session_public_id: string } =
    await transferred.json();
  await expect(page.getByText("Contexte transmis et à jour")).toBeVisible();
  const technical = await generate(
    page,
    project.public_id,
    handoff.target_session_public_id,
    "technical_tickets",
  );
  const lead = await reviewAndValidate(
    page,
    await authoredTickets(page, request, technical, 3, "Technique"),
    3,
  );
  const techJobs = await publishAndRecover(
    page,
    request,
    lead,
    3,
    "github",
    true,
  );
  // Removing from the new draft never changes the old version/index receipts.
  await page.goto(`/artifacts/${lead.artifact.public_id}`);
  await page.getByRole("button", { name: "Nouvelle révision" }).click();
  await page.getByRole("button", { name: "Retirer le ticket 2" }).click();
  await expect(
    page.getByRole("textbox", { name: "Titre du ticket 2", exact: true }),
  ).toBeFocused();
  await expect(
    page.getByRole("textbox", { name: "Titre du ticket 2", exact: true }),
  ).toHaveValue("[FICTIF] Technique 3");
  const removal = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      r.url().endsWith(`/artifacts/${lead.artifact.public_id}/draft`),
  );
  await page
    .getByRole("button", { name: "Enregistrer la nouvelle version" })
    .click();
  const removalResponse = await removal;
  expect(removalResponse.status()).toBe(200);
  const reduced = (await removalResponse.json()) as ArtifactDetail;
  const reducedDraft = typedDraft(reduced.current_version.structured_content)!;
  expect(reduced.current_version.version).toBe(4);
  expect(reducedDraft.tickets).toEqual(
    typedDraft(lead.current_version.structured_content)!.tickets.filter(
      (_, i) => i !== 1,
    ),
  );
  expect(reduced.current_version.sources).toEqual(lead.current_version.sources);
  const sourceLink = page
    .getByRole("region", { name: "Suivi des tickets demandés" })
    .getByRole("link", { name: "Ouvrir la version source · ticket 3" });
  await expect(sourceLink).toHaveAttribute(
    "href",
    `/artifacts/${lead.artifact.public_id}?version=${lead.current_version.public_id}&ticket=2`,
  );
  await sourceLink.click();
  await expect(
    page.getByRole("article", { name: "Ticket 3 · version 3", exact: true }),
  ).toBeFocused();
  // The newest version needs a fresh human validation and T16 extra-issue warning.
  await page.goto(`/artifacts/${lead.artifact.public_id}`);
  const validateReduced = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      r.url().endsWith(`/artifacts/${lead.artifact.public_id}/validate`),
  );
  await page.getByRole("button", { name: "Valider cette version" }).click();
  expect((await validateReduced).status()).toBe(200);
  await expect(page.getByText(/Version 5 · Validée/)).toBeVisible();
  await page
    .getByRole("button", { name: "Préparer une autre sélection" })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Sélectionner Ticket 2 — [FICTIF] Technique 3",
      exact: true,
    })
    .check();
  await page.getByRole("button", { name: "Préparer les 1 tickets" }).click();
  const extraPreview = page.getByRole("region", {
    name: "Confirmation des tickets",
  });
  await expect(extraPreview).toContainText(
    "Les nouvelles issues s’ajouteront aux anciennes.",
  );
  const extraConfirm = extraPreview.getByRole("button", {
    name: "Confirmer la création de 1 tickets dans GitHub",
    exact: true,
  });
  await expect(extraConfirm).toBeDisabled();
  await extraPreview
    .getByRole("checkbox", {
      name: /Je confirme la création de nouveaux tickets/,
    })
    .check();
  await expect(extraConfirm).toBeEnabled();
  // Do not create a sixth issue: preview and acknowledgment are local preparation.
  const remaining = await get<{ items: Publication[] }>(
    request,
    `/api/artifacts/${lead.artifact.public_id}/publications`,
  );
  expect(remaining.items.map((job) => job.public_id).sort()).toEqual(
    techJobs.map((job) => job.public_id).sort(),
  );
  await page.goto(
    `/artifacts/${lead.artifact.public_id}?version=${lead.current_version.public_id}&ticket=2`,
  );
  await expect(
    page.getByRole("article", { name: "Ticket 3 · version 3", exact: true }),
  ).toBeFocused();
  await page.setViewportSize({ width: 390, height: 844 });
  // Viewport metrics arrive before the responsive CSS transitions are painted.
  // Even reduced-motion transitions need a frame; measure the committed layout.
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
      }),
  );
  const mobileLayout = await page.evaluate(() => ({
    contentWidth: document.documentElement.scrollWidth,
    viewportWidth: innerWidth,
  }));
  expect(
    mobileLayout.contentWidth,
    `Le contenu mesure ${mobileLayout.contentWidth}px pour un écran de ${mobileLayout.viewportWidth}px`,
  ).toBeLessThanOrEqual(mobileLayout.viewportWidth);
  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(axe.violations).toEqual([]);
  await page.screenshot({
    path: testInfo.outputPath("ticket-real-mobile.png"),
    fullPage: false,
  });
  await testInfo.attach("fixture-preparation.json", {
    body: JSON.stringify(
      {
        agent_mode: "deterministic",
        generated_tickets_per_document: 1,
        authored_fictitious_tickets: { product: 2, technical: 3 },
        preparation: "human_revision_editor",
        added_ticket_source_ids: [],
        removed_ticket_index_in_new_revision: 1,
        historical_issue_version: 3,
        product_publication_ids: productJobs.map((job) => job.public_id),
        technical_publication_ids: techJobs.map((job) => job.public_id),
      },
      null,
      2,
    ),
    contentType: "application/json",
  });
  expect(runtimeErrors).toEqual([]);
});
