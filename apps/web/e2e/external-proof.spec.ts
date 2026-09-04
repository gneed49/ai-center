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
