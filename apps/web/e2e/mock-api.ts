import type { Page, Request } from "@playwright/test";

const now = "2026-08-25T10:00:00.000Z";

export const ids = {
  project: "10000000-0000-4000-8000-000000000001",
  otherProject: "10000000-0000-4000-8000-000000000002",
  productSession: "20000000-0000-4000-8000-000000000001",
  techSession: "20000000-0000-4000-8000-000000000002",
  pack: "30000000-0000-4000-8000-000000000001",
  handoff: "40000000-0000-4000-8000-000000000001",
  insight: "50000000-0000-4000-8000-000000000001",
  knowledge: "60000000-0000-4000-8000-000000000001",
  version: "70000000-0000-4000-8000-000000000001",
  message: "80000000-0000-4000-8000-000000000001",
  deliverable: "90000000-0000-4000-8000-000000000001",
  section: "91000000-0000-4000-8000-000000000001",
  reference: "92000000-0000-4000-8000-000000000001",
  connection: "93000000-0000-4000-8000-000000000001",
  evidence: "94000000-0000-4000-8000-000000000001",
};

export type MockApiOptions = {
  packStatus?: "current" | "stale";
  packGraphVersion?: number;
  graphVersion?: number;
  gateGraphVersion?: number;
  latestHandoff?: boolean;
  sessionHasPack?: boolean;
  coverageFailure?: boolean;
  projectCreateFailsOnce?: boolean;
  projectCreateDelayMs?: number;
  messageFailsOnce?: boolean;
  messageDelayMs?: number;
  messagePermanentFailure?: boolean;
  messageResponseLostOnce?: boolean;
  gateFailsOnce?: boolean;
  externalProof?: boolean;
};

export type MockApiState = {
  requests: Request[];
  projectCreateAttempts: number;
  messageAttempts: number;
  messageProviderAttempts: number;
  messageMutations: number;
  gateAttempts: number;
  latestHandoff: ReturnType<typeof buildHandoff> | null;
  externalImported: boolean;
  externalHeadSha: string;
  evidenceStatus: "candidate" | "valid" | "stale" | "rejected" | null;
};

export function buildContextPack(
  status: "current" | "stale" = "current",
  sourceGraphVersion = status === "current" ? 4 : 3,
) {
  return {
    public_id: ids.pack,
    version: 2,
    status,
    source_graph_version: sourceGraphVersion,
    compiler_version: "context-compiler/0.2.0",
    selection_mode: "hybrid",
    content_hash:
      "ab78a0b232e2110c0dc6f9d01aa112a52426c499681636b6d14d2a62d3f253c4",
    token_budget: 12_000,
    token_count: 3_200,
    compiled_at: now,
    invalidated_at: status === "stale" ? now : null,
    stale_reason:
      status === "stale" ? "Une connaissance source a été révisée." : null,
    content: {
      objective: "Prouver la continuité du contexte jusqu’à GitHub",
      knowledge: [
        {
          knowledge_public_id: ids.knowledge,
          version_public_id: ids.version,
          version_number: 1,
          entry_type: "requirement",
          title: "GitHub reste la source canonique",
          statement: "AI Center importe les preuves sans écrire dans GitHub.",
          rationale: "Préserver les outils en place.",
          node_key: "product",
        },
      ],
      provenance: [
        {
          knowledge_public_id: ids.knowledge,
          version_public_id: ids.version,
          reason_code: "contract_required",
          explanation: "Exigence obligatoire pour le handoff Tech.",
        },
      ],
    },
    selection_items: [
      {
        candidate_public_id: ids.version,
        decision: "included",
        reason_code: "contract_required",
        explanation: "Exigence obligatoire pour le handoff Tech.",
        rank: 1,
        estimated_tokens: 180,
        is_mandatory: true,
      },
      {
        candidate_public_id: "70000000-0000-4000-8000-000000000002",
        decision: "excluded",
        reason_code: "not_selected",
        explanation: "Sans effet sur la tâche de preuve GitHub.",
        rank: 12,
        estimated_tokens: 90,
        is_mandatory: false,
      },
    ],
  } as const;
}

function buildHandoff(pack = buildContextPack()) {
  return {
    public_id: ids.handoff,
    context_pack_public_id: pack.public_id,
    target_session_public_id: ids.techSession,
    status: "completed",
    context_pack: pack,
    completed_at: now,
  };
}

function buildSnapshot(
  graphVersion = 4,
  gateGraphVersion = graphVersion,
  externalProof = false,
) {
  return {
    project: {
      public_id: ids.project,
      name: "AI Center",
      objective: "Prouver la continuité du contexte jusqu’à GitHub",
      summary: "Control plane contextuel",
      status: "active",
      graph_version: graphVersion,
      created_at: now,
      updated_at: now,
    },
    nodes: [
      {
        public_id: "11000000-0000-4000-8000-000000000001",
        node_key: "product",
        title: "Produit",
        description: "Décisions et exigences",
        summary: "Produit",
        profile_key: "product",
        profile_name: "Product Agent",
        scope_kind: "product",
      },
      {
        public_id: "11000000-0000-4000-8000-000000000002",
        node_key: "tech",
        title: "Tech",
        description: "Projection du contexte confirmé",
        summary: "Tech",
        profile_key: "tech",
        profile_name: "Tech Agent",
        scope_kind: "tech",
      },
    ],
    sessions: [
      buildSession("product", ids.productSession),
      buildSession("tech", ids.techSession),
    ],
    knowledge: [
      {
        public_id: ids.knowledge,
        version_public_id: ids.version,
        version_number: 1,
        entry_type: "requirement",
        title: "GitHub reste la source canonique",
        statement: "AI Center importe les preuves sans écrire dans GitHub.",
        rationale: "Préserver les outils en place.",
        node_key: "product",
        created_at: now,
      },
    ],
    deliverables: externalProof
      ? [
          {
            public_id: ids.deliverable,
            deliverable_type: "technical-delivery-plan",
            title: "Plan de preuve externe",
            summary: "Une projection contrôlée par le ContextPack.",
            content: {
              validation: ["Relier une PR réelle à l’exigence confirmée."],
            },
            status: "committed",
            coverage_status: "missing",
            version: 1,
            updated_at: now,
          },
        ]
      : [],
    insights: [buildInsight(graphVersion)],
    gate: {
      public_id: "12000000-0000-4000-8000-000000000001",
      status: "passed",
      graph_version: gateGraphVersion,
      missing: [],
      warnings: [],
      counts: {
        business_rules: 1,
        requirements: 1,
        acceptance_criteria: 1,
        open_questions: 0,
      },
    },
    latest_handoff: null,
  };
}

function buildCoverage(status: "missing" | "covered" = "missing") {
  return {
    total: 1,
    covered: status === "covered" ? 1 : 0,
    partial: 0,
    missing: status === "missing" ? 1 : 0,
    items: [
      {
        requirement_public_id: ids.knowledge,
        requirement_version_public_id: ids.version,
        requirement_title: "GitHub reste la source canonique",
        requirement_statement:
          "AI Center importe les preuves sans écrire dans GitHub.",
        deliverable_public_id: ids.deliverable,
        section_public_id: ids.section,
        evidence_public_id: status === "covered" ? ids.evidence : null,
        status,
        explanation:
          status === "covered"
            ? "Preuve GitHub validée humainement."
            : "Aucune preuve externe valide.",
      },
    ],
  };
}

function buildExternalReference(
  headSha: string,
  evidenceStatus: "candidate" | "valid" | "stale" | "rejected" | null,
) {
  return {
    public_id: ids.reference,
    project_public_id: ids.project,
    tool_connection_public_id: ids.connection,
    provider: "github",
    object_kind: "pull_request",
    external_id: "gneed49/ai-center#42",
    canonical_url: "https://github.com/gneed49/ai-center/pull/42",
    repository_full_name: "gneed49/ai-center",
    display_title: "PR #42 — preuve ContextPack",
    sync_status: "current",
    etag: '"proof-etag"',
    last_synced_at: now,
    created_at: now,
    updated_at: now,
    latest_observation: {
      public_id: "95000000-0000-4000-8000-000000000001",
      observation_status: "current",
      content_hash:
        "7f78a0b232e2110c0dc6f9d01aa112a52426c499681636b6d14d2a62d3f253c",
      etag: '"proof-etag"',
      observed_state: {
        state: "open",
        base_sha: "a".repeat(40),
        head_sha: headSha,
        checks: [
          { name: "desktop-ci", status: "completed", conclusion: "success" },
        ],
      },
      provider_updated_at: now,
      observed_at: now,
    },
    evidences: evidenceStatus
      ? [
          {
            public_id: ids.evidence,
            requirement_public_id: ids.knowledge,
            requirement_version_public_id: ids.version,
            deliverable_public_id: ids.deliverable,
            deliverable_section_public_id: ids.section,
            evidence_type: "github_pull_request",
            title: "PR observée — GitHub reste la source canonique",
            description:
              "Preuve candidate issue d’une observation GitHub read-only.",
            source_reference: `github:gneed49/ai-center#42@${headSha}`,
            status: evidenceStatus,
            created_at: now,
          },
        ]
      : [],
  } as const;
}

function buildSession(nodeKey: "product" | "tech", id: string) {
  return {
    public_id: id,
    title: nodeKey === "tech" ? "Plan de livraison Tech" : "Cadrage Produit",
    status: "active",
    node_key: nodeKey,
    scope_kind: nodeKey,
    created_at: now,
    updated_at: now,
  };
}

function buildSessionView(pack: ReturnType<typeof buildContextPack> | null) {
  return {
    session: buildSession("tech", ids.techSession),
    messages: [],
    proposals: [],
    context_pack: pack,
  };
}

function buildInsight(graphVersion = 4, status = "open") {
  return {
    public_id: ids.insight,
    project_public_id: ids.project,
    project_name: "AI Center",
    project_graph_version: graphVersion,
    insight_type: "contradiction",
    status,
    severity: "blocking",
    confidence: 0.91,
    title: "Rétention contradictoire",
    explanation: "Deux règles confirmées décrivent une rétention incompatible.",
    resolution_justification: status === "resolved" ? "Règle révisée" : null,
    detected_at: now,
    updated_at: now,
  };
}

function buildInsightDetail(graphVersion = 4, status = "open") {
  return {
    insight: buildInsight(graphVersion, status),
    sources: [
      {
        source_role: "product_rule",
        object_kind: "knowledge_entry_version",
        object_public_id: ids.version,
        knowledge_public_id: ids.knowledge,
        version_public_id: ids.version,
        version_title: "Politique de conservation",
        version_statement: "Les données sont conservées sans expiration.",
      },
    ],
  };
}

export async function installMockApi(page: Page, options: MockApiOptions = {}) {
  const graphVersion = options.graphVersion ?? 4;
  const initialPack = buildContextPack(
    options.packStatus ?? "current",
    options.packGraphVersion ??
      (options.packStatus === "stale" ? graphVersion - 1 : graphVersion),
  );
  const state: MockApiState = {
    requests: [],
    projectCreateAttempts: 0,
    messageAttempts: 0,
    messageProviderAttempts: 0,
    messageMutations: 0,
    gateAttempts: 0,
    latestHandoff:
      options.latestHandoff === false ? null : buildHandoff(initialPack),
    externalImported: false,
    externalHeadSha: "b".repeat(40),
    evidenceStatus: null,
  };
  let insightStatus = "open";
  const messageResults = new Map<string, { body: unknown; status: number }>();

  await page.route("http://127.0.0.1:4317/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    state.requests.push(request);

    const json = (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });

    if (path === "/api/projects" && request.method() === "GET") return json([]);

    if (path === "/api/projects" && request.method() === "POST") {
      state.projectCreateAttempts += 1;
      if (options.projectCreateDelayMs)
        await new Promise((resolve) =>
          setTimeout(resolve, options.projectCreateDelayMs),
        );
      if (options.projectCreateFailsOnce && state.projectCreateAttempts === 1)
        return json(
          {
            message: "Création temporairement indisponible",
            code: "provider_unavailable",
            retryable: true,
          },
          503,
        );
      const input = request.postDataJSON() as {
        name: string;
        objective: string;
      };
      return json({
        ...buildSnapshot(graphVersion).project,
        name: input.name,
        objective: input.objective,
      });
    }

    if (path === `/api/projects/${ids.project}/snapshot`)
      return json(
        buildSnapshot(
          graphVersion,
          options.gateGraphVersion ?? graphVersion,
          options.externalProof,
        ),
      );

    if (path.endsWith("/snapshot"))
      return json({ message: "Projet introuvable", code: "not_found" }, 404);

    if (
      path === `/api/projects/${ids.project}/gates/product-ready/evaluate` &&
      request.method() === "POST"
    ) {
      state.gateAttempts += 1;
      if (options.gateFailsOnce && state.gateAttempts === 1)
        return json(
          {
            message: "Évaluation temporairement indisponible",
            code: "provider_unavailable",
            retryable: true,
          },
          503,
        );
      return json(
        buildSnapshot(graphVersion, graphVersion, options.externalProof).gate,
      );
    }

    if (path === `/api/projects/${ids.project}/handoffs/latest`)
      return json(state.latestHandoff);

    if (
      path === `/api/projects/${ids.project}/context-packs` &&
      request.method() === "POST"
    ) {
      const pack = buildContextPack("current", graphVersion);
      return json(pack);
    }

    if (
      path === `/api/projects/${ids.project}/handoffs` &&
      request.method() === "POST"
    ) {
      state.latestHandoff = buildHandoff(
        buildContextPack("current", graphVersion),
      );
      return json(state.latestHandoff);
    }

    if (
      path === `/api/projects/${ids.project}/sessions/${ids.techSession}` &&
      request.method() === "GET"
    )
      return json(
        buildSessionView(options.sessionHasPack === false ? null : initialPack),
      );

    if (path.includes("/sessions/") && request.method() === "GET")
      return json({ message: "Session hors projet", code: "not_found" }, 404);

    if (
      path ===
        `/api/projects/${ids.project}/sessions/${ids.techSession}/messages` &&
      request.method() === "POST"
    ) {
      state.messageAttempts += 1;
      const requestBody = request.postDataJSON() as {
        content?: string;
        client_message_id?: string;
      };
      if (options.messageDelayMs)
        await new Promise((resolve) =>
          setTimeout(resolve, options.messageDelayMs),
        );
      const idempotencyKey =
        request.headers()["idempotency-key"] ??
        requestBody.client_message_id ??
        "missing-idempotency-key";
      const stored = messageResults.get(idempotencyKey);
      if (stored) return json(stored.body, stored.status);

      state.messageProviderAttempts += 1;
      if (options.messageFailsOnce && state.messageProviderAttempts === 1)
        return json(
          {
            message: "Fournisseur temporairement indisponible",
            code: "provider_unavailable",
            retryable: true,
          },
          503,
        );

      if (options.messagePermanentFailure) {
        const failure = {
          message: "Commande refusée de manière permanente",
          code: "invalid_request",
          retryable: false,
        };
        messageResults.set(idempotencyKey, { body: failure, status: 422 });
        return json(failure, 422);
      }

      const response = {
        ...buildSessionView(initialPack),
        messages: [
          {
            public_id: ids.message,
            role: "user",
            content: requestBody.content ?? "Prépare le plan de preuve GitHub",
            agent_scope: null,
            metadata: {},
            created_at: now,
          },
        ],
      };
      state.messageMutations += 1;
      messageResults.set(idempotencyKey, { body: response, status: 200 });
      if (options.messageResponseLostOnce && state.messageMutations === 1)
        return route.abort("timedout");
      return json(response);
    }

    if (path === `/api/projects/${ids.project}/coverage`) {
      if (options.coverageFailure)
        return json(
          {
            message: "Calcul de couverture indisponible",
            code: "coverage_unavailable",
          },
          503,
        );
      return json(
        options.externalProof
          ? buildCoverage(
              state.evidenceStatus === "valid" ? "covered" : "missing",
            )
          : { total: 1, covered: 0, partial: 1, missing: 0, items: [] },
      );
    }

    if (path === `/api/projects/${ids.project}/external-references`) {
      if (request.method() === "GET")
        return json(
          state.externalImported
            ? [
                buildExternalReference(
                  state.externalHeadSha,
                  state.evidenceStatus,
                ),
              ]
            : [],
        );
      state.externalImported = true;
      return json(
        buildExternalReference(state.externalHeadSha, state.evidenceStatus),
      );
    }

    if (path === `/api/external-references/${ids.reference}`)
      return json(
        buildExternalReference(state.externalHeadSha, state.evidenceStatus),
      );

    if (path === `/api/external-references/${ids.reference}/evidence`) {
      state.evidenceStatus = "candidate";
      return json(
        buildExternalReference(state.externalHeadSha, state.evidenceStatus)
          .evidences[0],
      );
    }

    if (
      path ===
      `/api/external-references/${ids.reference}/evidence/${ids.evidence}/review`
    ) {
      const body = request.postDataJSON() as {
        decision: "validate" | "reject";
      };
      state.evidenceStatus =
        body.decision === "validate" ? "valid" : "rejected";
      return json(
        buildExternalReference(state.externalHeadSha, state.evidenceStatus)
          .evidences[0],
      );
    }

    if (path === `/api/external-references/${ids.reference}/refresh`) {
      state.externalHeadSha = "c".repeat(40);
      if (state.evidenceStatus === "valid") state.evidenceStatus = "stale";
      return json(
        buildExternalReference(state.externalHeadSha, state.evidenceStatus),
      );
    }

    if (path === "/api/insights") return json([buildInsight(graphVersion)]);

    if (
      path === `/api/projects/${ids.project}/insights/${ids.insight}` &&
      request.method() === "GET"
    )
      return json(buildInsightDetail(graphVersion, insightStatus));

    if (
      path === `/api/projects/${ids.project}/insights/${ids.insight}/resolve` &&
      request.method() === "POST"
    ) {
      insightStatus = "resolved";
      return json(buildInsightDetail(graphVersion + 1, insightStatus));
    }

    return json(
      { message: `Route mock absente: ${path}`, code: "not_found" },
      404,
    );
  });

  return state;
}

export function captureConsoleErrors(page: Page) {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  return errors;
}
