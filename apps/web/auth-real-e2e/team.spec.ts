import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";

test("partage un projet entre deux comptes Auth et révoque réellement l’accès", async ({
  browser,
  request,
}) => {
  const file = process.env.AI_CENTER_AUTH_BROWSER_FIXTURE;
  if (!file) throw new Error("Guarded private fixture required");
  const { sessions } = JSON.parse(await readFile(file, "utf8")) as {
    sessions: { access_token: string; user: { id: string } }[];
  };
  const origin = "http://127.0.0.1:5183";
  const api = "http://127.0.0.1:4618";
  const contexts = await Promise.all(
    sessions.map((session) =>
      browser.newContext({
        baseURL: origin,
        storageState: {
          cookies: [],
          origins: [
            {
              origin,
              localStorage: [
                { name: "sb-127-auth-token", value: JSON.stringify(session) },
              ],
            },
          ],
        },
      }),
    ),
  );
  try {
    const owner = await contexts[0].newPage();
    const colleague = await contexts[1].newPage();
    await owner.goto("/");
    await owner
      .getByLabel("Nom de l’entreprise")
      .fill("[FICTIF] Équipe Auth réelle");
    await owner
      .getByRole("button", { name: "Créer l’espace de mon entreprise" })
      .click();
    await expect(
      owner.getByRole("combobox", { name: "Entreprise", exact: true }),
    ).toContainText("[FICTIF] Équipe Auth réelle");
    const workspace = await owner
      .getByRole("combobox", { name: "Entreprise", exact: true })
      .inputValue();
    const memberHeaders = {
      Authorization: `Bearer ${sessions[1].access_token}`,
      "X-AI-Center-Workspace-Id": workspace,
    };
    await owner.goto("/company");
    await owner
      .getByLabel("Repère pour votre suivi")
      .fill("[FICTIF] Collègue lecteur");
    await owner.getByLabel("Rôle accordé").selectOption("viewer");
    await owner
      .getByRole("button", { name: "Créer le lien d’invitation" })
      .click();
    const invitation = new URL(
      await owner.getByLabel("Lien de cette invitation").inputValue(),
    );
    await colleague.goto(invitation.toString());
    await colleague
      .getByLabel("Votre nom dans l’équipe")
      .fill("[FICTIF] Collègue local");
    await colleague
      .getByRole("button", { name: "Accepter l’invitation" })
      .click();
    await expect(
      colleague.getByRole("combobox", { name: "Entreprise", exact: true }),
    ).toHaveValue(workspace);
    await owner.goto("/projects/new");
    await owner
      .getByLabel("Nom du projet")
      .fill("[FICTIF] Projet partagé réel");
    await owner.getByRole("button", { name: "Créer le projet" }).click();
    await expect(
      owner.getByRole("heading", { name: "[FICTIF] Projet partagé réel" }),
    ).toBeVisible();
    const projectPath = new URL(owner.url()).pathname;
    await colleague.goto(projectPath);
    await expect(
      colleague.getByRole("heading", { name: "[FICTIF] Projet partagé réel" }),
    ).toBeVisible();
    await colleague.reload();
    await expect(
      colleague.getByRole("heading", { name: "[FICTIF] Projet partagé réel" }),
    ).toBeVisible();
    const forbidden = await request.post(`${api}/api/projects`, {
      headers: { ...memberHeaders, "Idempotency-Key": crypto.randomUUID() },
      data: { name: "[FICTIF] Refus lecteur", objective: "" },
    });
    expect(forbidden.status()).toBe(403);
    const foreign = await request.get(`${api}/api/company`, {
      headers: {
        ...memberHeaders,
        "X-AI-Center-Workspace-Id": crypto.randomUUID(),
      },
    });
    expect(foreign.status()).toBe(403);
    await owner.goto("/company");
    const role = owner.getByRole("combobox", {
      name: "Rôle de [FICTIF] Collègue local",
    });
    await expect(role).toHaveValue("viewer");
    const promotion = owner.waitForResponse(
      (response) =>
        response.request().method() === "PATCH" &&
        response.url().includes("/api/company/members/"),
    );
    await role.selectOption("editor");
    expect((await promotion).status()).toBe(200);
    await expect(role).toBeEnabled();
    await expect(role).toHaveValue("editor");
    await colleague.goto("/projects/new");
    await colleague
      .getByLabel("Nom du projet")
      .fill("[FICTIF] Projet du collègue");
    await colleague.getByRole("button", { name: "Créer le projet" }).click();
    await expect(
      colleague.getByRole("heading", { name: "[FICTIF] Projet du collègue" }),
    ).toBeVisible();
    const removal = owner.waitForResponse(
      (response) =>
        response.request().method() === "PATCH" &&
        response.url().includes("/api/company/members/"),
    );
    await owner
      .getByRole("button", {
        name: "Retirer l’accès de [FICTIF] Collègue local",
      })
      .click();
    expect((await removal).status()).toBe(200);
    await expect(
      owner.getByRole("button", {
        name: "Retirer l’accès de [FICTIF] Collègue local",
      }),
    ).toBeDisabled();
    const revoked = await request.get(`${api}/api/company`, {
      headers: memberHeaders,
    });
    expect(revoked.status()).toBe(403);
    const token = new URLSearchParams(invitation.hash.slice(1)).get("token");
    const oldInvite = await request.post(
      `${api}/api/team/invitations/${invitation.pathname.split("/").at(-1)}/accept`,
      {
        headers: { Authorization: memberHeaders.Authorization },
        data: { token, display_name: "[FICTIF] Retour refusé" },
      },
    );
    expect(oldInvite.status()).toBe(404);
    const reloadedAccess = colleague.waitForResponse(
      (response) =>
        response.request().method() === "GET" &&
        response.url().includes("/api/projects/") &&
        response.status() === 403,
    );
    await colleague.reload();
    await reloadedAccess;
    await expect(
      colleague.getByRole("alert").getByText("Accès refusé dans ce contexte"),
    ).toBeVisible();
    await expect(
      colleague.getByRole("heading", { name: "[FICTIF] Projet du collègue" }),
    ).toHaveCount(0);
    await expect(
      colleague.getByRole("heading", { name: "[FICTIF] Projet partagé réel" }),
    ).toHaveCount(0);
  } finally {
    await Promise.all(contexts.map((context) => context.close()));
  }
});
