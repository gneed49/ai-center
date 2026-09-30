import type { Page, Request } from "@playwright/test";
import type {
  SourceCommandReceipt,
  SourceCommandResult,
  ToolSourceDetail,
} from "../src/api/tool-source-types";
import {
  fixtureSourceObservation,
  fixtureSourceReference,
} from "../src/test-fixtures/tool-sources";
import {
  fixtureTools,
  fixtureToolConnection,
} from "../src/test-fixtures/work-tools";
import { ids, installMockApi } from "./mock-api";

// [FICTIF] API doubles. No remote tool or provider is called by these scenarios.
export const sourceIds = {
  linearConnection: "60000000-0000-4000-8000-000000000001",
  notionConnection: "60000000-0000-4000-8000-000000000002",
  reference: fixtureSourceReference.public_id,
  original: fixtureSourceObservation.observation.public_id,
  current: "40000000-0000-4000-8000-000000000002",
  publication: "40000000-0000-4000-8000-000000000003",
};
export async function installSourceApi(
  page: Page,
  options: {
    attached?: boolean;
    longTitle?: boolean;
    lostResponse?: boolean;
    role?: "owner" | "viewer";
    current?: "partial" | "unavailable";
  } = {},
) {
  await installMockApi(page, { companySuite: true });
  const original = structuredClone(fixtureSourceObservation);
  original.observation.source_project_public_id = ids.project;
  const current = structuredClone(original);
  if (options.current) {
    original.observation.freshness = {
      ...original.observation.freshness,
      is_current: false,
      eligible: false,
      reasons: ["historical"],
      current_observation_id: sourceIds.current,
    };
    current.observation = {
      ...current.observation,
      public_id: sourceIds.current,
      version: 2,
      observed_at: "2026-09-24T01:00:00Z",
      coverage: options.current === "partial" ? "partial" : "none",
      availability: options.current === "partial" ? "available" : "unavailable",
      omission_reasons:
        options.current === "partial"
          ? ["local_text_limit", "comments_not_read"]
          : ["remote_unavailable"],
      freshness: {
        ...current.observation.freshness,
        current_observation_id: sourceIds.current,
        eligible: options.current === "partial",
        reasons: options.current === "partial" ? [] : ["unavailable"],
      },
    };
    current.body_markdown =
      options.current === "partial"
        ? "[FICTIF] Lecture actuelle partielle : politique de crédits modifiée."
        : "";
    current.observation.excerpt = current.body_markdown;
  }
  if (options.longTitle)
    current.observation.title = `[FICTIF] ${"Documentation".repeat(35)}`;
  let source: ToolSourceDetail = {
    reference: {
      ...fixtureSourceReference,
      project_id: ids.project,
      current_observation_id: current.observation.public_id,
    },
    observation: current.observation,
  };
  let attached = options.attached ?? false;
  const requests: Request[] = [];
  const receipts = new Map<string, SourceCommandReceipt>();
  let lost = options.lostResponse ?? false;
  let denyExact = false;
  let reads = 0;
  await page.route("http://127.0.0.1:4317/api/**", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    const json = (body: unknown, status = 200) =>
      route.fulfill({ status, json: body });
    if (path === "/api/work-tools")
      return json({
        ...fixtureTools,
        connections: ["linear", "notion"].map((provider, index) => ({
          ...fixtureToolConnection,
          provider,
          public_id: index
            ? sourceIds.notionConnection
            : sourceIds.linearConnection,
          name: index
            ? "[FICTIF] Documentation Notion"
            : "[FICTIF] Équipe Linear",
          allow_existing_reads: true,
        })),
        capabilities: ["linear", "notion"].map((provider) => ({
          provider,
          create: true,
          read: true,
          read_existing: true,
          reconcile: true,
          update: false,
        })),
      });
    if (
      !path.includes("tool-source") &&
      !path.includes("publication-observations")
    )
      return route.fallback();
    requests.push(request);
    if (path.includes("/tool-source-commands/")) {
      const key = path.split("/").at(-1)!;
      return json(
        receipts.get(key) ?? {
          status: "not_received",
          can_retry: true,
          retry_after: null,
          result: null,
          error: null,
        },
      );
    }
    if (path === `/api/projects/${ids.project}/tool-sources`) {
      if (request.method() === "GET")
        return json({
          items:
            attached &&
            (!url.searchParams.get("provider") ||
              url.searchParams.get("provider") === source.reference.provider)
              ? [source]
              : [],
          next_cursor: null,
          total_count: attached ? 1 : 0,
          active_count: attached ? 1 : 0,
          limit: 25,
        });
      const input = request.postDataJSON(),
        key = (await request.headerValue("Idempotency-Key"))!;
      reads++;
      if (input.provider === "notion") {
        current.observation = {
          ...current.observation,
          provider: "notion",
          object_kind: "page",
          title: "[FICTIF] Cahier produit Notion",
          canonical_url:
            "https://www.notion.so/70000000000040008000000000000001",
        };
        current.metadata = {};
        current.body_markdown =
          "[FICTIF] Le cahier Notion reste dans son outil d’origine.";
        source = {
          reference: {
            ...source.reference,
            provider: "notion",
            object_kind: "page",
            connection_id: sourceIds.notionConnection,
            canonical_url: current.observation.canonical_url,
          },
          observation: current.observation,
        };
      }
      const result: SourceCommandResult = {
        action: "attach",
        effect: "created",
        verification_status: "available",
        ...source,
      };
      attached = true;
      receipts.set(key, {
        status: "completed",
        can_retry: false,
        retry_after: null,
        result,
        error: null,
      });
      if (lost) {
        lost = false;
        return route.abort("failed");
      }
      return json(result);
    }
    if (path === `/api/tool-sources/${sourceIds.reference}`)
      return json(source);
    if (path.endsWith("/observations"))
      return json({
        items: options.current
          ? [current.observation, original.observation]
          : [current.observation],
        next_cursor: null,
        total_count: options.current ? 2 : 1,
        limit: 25,
      });
    if (path.includes("/tool-source-observations/")) {
      if (denyExact)
        return json(
          { code: "forbidden", message: "[FICTIF] Votre accès a été retiré." },
          403,
        );
      return json(
        path.endsWith(current.observation.public_id) ? current : original,
      );
    }
    if (path.includes("/publication-observations/"))
      return json({
        ...current,
        observation: {
          ...current.observation,
          public_id: sourceIds.publication,
          source_kind: "publication_observation",
          reference_public_id: null,
          publication_public_id: "50000000-0000-4000-8000-000000000001",
        },
      });
    return json(
      {
        code: "fixture_unhandled",
        message: "[FICTIF] Route non prévue par ce scénario.",
      },
      500,
    );
  });
  if (options.role === "viewer")
    await page.route("http://127.0.0.1:4317/api/company", async (route) => {
      // The company suite already supplies this contract; copy only the role override.
      const { fixtureCompany } =
        await import("../src/test-fixtures/company-context");
      return route.fulfill({
        json: {
          ...fixtureCompany,
          workspace: {
            ...fixtureCompany.workspace,
            public_id: "01000000-0000-4000-8000-000000000001",
            role: "viewer",
          },
          projects: [
            {
              ...fixtureCompany.projects[0],
              public_id: ids.project,
              name: "[FICTIF] Projet partagé",
              status: "active",
            },
          ],
        },
      });
    });
  return {
    requests,
    get reads() {
      return reads;
    },
    denyExact: () => {
      denyExact = true;
    },
  };
}
