import type { Route } from "@playwright/test";
import type { ProjectSummary } from "../src/api/types";
import type {
  ArtifactDetail,
  ArtifactVersion,
  CreateArtifact,
  SaveArtifact,
} from "../src/api/artifact-types";
import type { TeamInvitation } from "../src/api/team-types";
import type {
  WorkToolConnection,
  Publication,
} from "../src/api/work-tool-types";
import { fixtureCompany } from "../src/test-fixtures/company-context";
import { fixtureSnapshot } from "../src/test-fixtures/context-loop";
import {
  fixtureArtifact,
  fixtureOldArtifactVersion,
} from "../src/test-fixtures/artifacts";
import { fixtureCode } from "../src/test-fixtures/code-evidence";
import {
  fixtureTools,
  fixtureToolConnection,
  fixturePublication,
} from "../src/test-fixtures/work-tools";

/** Synthetic state only. This adapter never contacts GitHub, email, Notion or Linear. */
export function companyMockHandler(project: ProjectSummary, enabled: boolean) {
  const workspaceId = "01000000-0000-4000-8000-000000000001";
  const companyScope = "01000000-0000-4000-8000-000000000002";
  const now = "2026-09-21T10:00:00Z";
  const storedProject = { ...project };
  const members = [
    {
      public_id: "fixture-member",
      actor_id: "local-actor",
      role: "owner",
      invitation_status: "accepted",
      accepted_at: now,
      display_name: "[FICTIF] Camille",
    },
  ];
  const invitations: TeamInvitation[] = [];
  const connections: WorkToolConnection[] = enabled
    ? [
        {
          ...fixtureToolConnection,
          provider: "github",
          name: "[FICTIF] Dépôt de test",
        },
      ]
    : [];
  const publications: Publication[] = [];
  let automation = {
    enabled: true,
    generation: 1,
    limits: {
      calls_per_hour: 100,
      concurrent_calls: 3,
      call_timeout_seconds: 60,
    },
    calls_last_hour: 2,
    active_calls: 0,
    known_estimated_cost_usd: null,
    runs_without_cost: 2,
  };
  let document: ArtifactDetail = {
    ...fixtureArtifact,
    artifact: { ...fixtureArtifact.artifact, project_id: project.public_id },
    current_version: { ...fixtureArtifact.current_version, sources: [] },
  };
  let versions: ArtifactVersion[] = [
    document.current_version,
    { ...fixtureOldArtifactVersion },
  ];
  let destination = {
    artifact_type: "specification",
    provider: "github",
    target_id: "fiction/example",
    label: "[FICTIF] Issues de recette",
    origin: "company",
    revision: 1,
    project_id: null as string | null,
  };
  let code = {
    ...fixtureCode,
    corpus: { ...fixtureCode.corpus, project_id: project.public_id },
  };
  return async (route: Route): Promise<boolean> => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname,
      method = request.method();
    const body = () => request.postDataJSON();
    const respond = async (value: unknown, status = 200) => {
      await route.fulfill({ status, json: value });
      return true;
    };
    if (path === "/api/workspaces/capabilities")
      return respond({ can_create_company: true });
    if (path === "/api/company/steward" && method === "GET")
      return respond({
        progress: {
          status: "idle",
          last_error_code: null,
          updated_at: now,
          lease_expired: null,
        },
        available_sources: 3,
        pending_sources: 0,
        examined_pairs: 2,
        omitted_neighbors: 0,
        exhaustive: false,
        max_sources: 10_000,
        max_neighbors_per_source: 12,
        max_provider_calls_per_hour: 6,
        coverage_notice:
          "[FICTIF] Un parcours terminé ne certifie pas l’absence de contradictions.",
      });
    if (enabled && path === "/api/company")
      return respond({
        ...fixtureCompany,
        workspace: { ...fixtureCompany.workspace, public_id: workspaceId },
        company_scope: { kind: "company", project_public_id: companyScope },
        projects: storedProject.status === "active" ? [storedProject] : [],
        members,
        agents: fixtureCompany.agents.map((agent) => ({
          ...agent,
          project_public_id:
            agent.scope_kind === "company" ? companyScope : project.public_id,
        })),
      });
    if (path === "/api/team/members") return respond(members);
    if (path === "/api/team/profile") {
      members[0].display_name = body().display_name;
      return respond(members[0]);
    }
    if (path === "/api/team/invitations") {
      if (method === "GET") return respond(invitations);
      const input = body();
      const item: TeamInvitation = {
        public_id: input.public_id,
        role: input.role,
        label: input.label,
        status: "pending",
        expires_at: "2099-09-28T10:00:00Z",
        created_at: now,
        accepted_at: null,
      };
      if (!invitations.some((i) => i.public_id === item.public_id))
        invitations.push(item);
      return respond(item);
    }
    if (path.match(/^\/api\/team\/invitations\/[^/]+\/revoke$/)) {
      const item = invitations.find((i) => i.public_id === path.split("/")[4]);
      if (!item) return respond({ message: "Invitation inconnue" }, 404);
      item.status = "revoked";
      return respond(item);
    }
    if (path.match(/^\/api\/team\/invitations\/[^/]+\/preview$/))
      return respond({
        public_id: path.split("/")[4],
        workspace_public_id: workspaceId,
        company_name: "[FICTIF] Atelier",
        role: "editor",
        expires_at: "2099-09-28T10:00:00Z",
      });
    if (path.match(/^\/api\/team\/invitations\/[^/]+\/accept$/))
      return respond({
        public_id: workspaceId,
        name: "[FICTIF] Atelier",
        role: "editor",
      });
    if (path === "/api/automation") {
      if (method === "POST") {
        const input = body();
        if (input.expected_generation !== automation.generation)
          return respond({ message: "État modifié", code: "conflict" }, 409);
        automation = {
          ...automation,
          enabled: input.enabled,
          generation: automation.generation + 1,
        };
      }
      return respond(automation);
    }
    if (path === "/api/company/projects/archived")
      return respond(
        storedProject.status === "archived" ? [storedProject] : [],
      );
    if (path === "/api/company/export")
      return respond({
        format: "ai-center-company-data-v1",
        workspace_public_id: workspaceId,
        generated_at: now,
        complete: true,
        record_count: 1,
        data: { projects: [storedProject] },
        excluded: ["credentials"],
      });
    if (path === `/api/projects/${project.public_id}/archive`) {
      storedProject.status = body().archived ? "archived" : "active";
      return respond(storedProject);
    }
    if (path === "/api/work-tools")
      return respond({
        ...fixtureTools,
        connections,
        capabilities: ["notion", "linear", "github"].map((provider) => ({
          provider,
          create: true,
          read: true,
          reconcile: true,
          update: false,
        })),
      });
    if (path === "/api/work-tools/connections") {
      const input = body();
      const previous = connections.find((c) => c.public_id === input.id);
      const value: WorkToolConnection = {
        public_id: input.id,
        provider: input.provider,
        name: input.name,
        enabled: true,
        revision: (previous?.revision ?? 0) + 1,
        created_at: now,
        updated_at: now,
      };
      if (previous) Object.assign(previous, value);
      else connections.push(value);
      return respond(value);
    }
    if (path.match(/^\/api\/work-tools\/connections\/[^/]+\/test$/))
      return respond({ ok: true, code: "credential_verified" });
    if (path.match(/^\/api\/work-tools\/connections\/[^/]+\/disable$/)) {
      const item = connections.find((c) => c.public_id === path.split("/")[4]);
      if (!item) return respond({ message: "Connexion inconnue" }, 404);
      item.enabled = false;
      item.revision++;
      return respond(item);
    }
    if (!enabled) return false;
    if (path === `/api/projects/${project.public_id}/export`)
      return respond({
        format: "ai-center-project-data-v1",
        workspace_public_id: workspaceId,
        project_public_id: project.public_id,
        generated_at: now,
        complete: true,
        record_count: 1,
        data: { projects: [storedProject] },
        referenced_sources: [],
        source_expansion: "direct_exact_versions_and_captured_origin_receipts",
        excluded: ["credentials"],
      });
    if (path === "/api/projects" && method === "GET")
      return respond(storedProject.status === "active" ? [storedProject] : []);
    if (path === `/api/projects/${companyScope}/snapshot`)
      return respond({
        ...fixtureSnapshot,
        project: {
          ...storedProject,
          public_id: companyScope,
          name: "[FICTIF] Contexte société",
        },
      });
    if (
      path === "/api/artifact-destinations" ||
      path === `/api/projects/${project.public_id}/artifact-destinations`
    ) {
      if (method === "POST") {
        const input = body();
        destination = {
          ...destination,
          ...input,
          revision: destination.revision + 1,
          origin: path.includes("/projects/") ? "project" : "company",
          project_id: path.includes("/projects/") ? project.public_id : null,
        };
        return respond({ items: [destination] });
      }
      return respond({ items: [destination] });
    }
    if (path.endsWith("/artifact-destinations/reset")) {
      destination = {
        ...destination,
        provider: "internal",
        target_id: "",
        origin: "default",
        revision: destination.revision + 1,
        project_id: null,
      };
      return respond({ items: [destination] });
    }
    if (
      path === `/api/projects/${project.public_id}/artifacts` ||
      path === `/api/projects/${companyScope}/artifacts`
    ) {
      if (method === "GET")
        return respond({
          items: [document.artifact],
          total: 1,
          limit: 25,
          offset: 0,
        });
      const input = body() as CreateArtifact;
      const scope = path.split("/")[3];
      const version = {
        ...document.current_version,
        ...input,
        public_id: crypto.randomUUID(),
        version: 1,
        status: "draft" as const,
        sources: [],
      };
      document = {
        artifact: {
          ...document.artifact,
          public_id: crypto.randomUUID(),
          project_id: scope,
          artifact_type: input.artifact_type,
          title: input.title,
          status: "draft",
          version: 1,
          current_version_id: version.public_id,
        },
        current_version: version,
      };
      versions = [version];
      return respond(document);
    }
    if (path === `/api/artifacts/${document.artifact.public_id}`)
      return respond(document);
    if (
      path === `/api/artifacts/${document.artifact.public_id}/draft` ||
      path === `/api/artifacts/${document.artifact.public_id}/validate`
    ) {
      const input = body() as SaveArtifact;
      if (input.expected_version_id !== document.current_version.public_id)
        return respond({ message: "Version modifiée", code: "conflict" }, 409);
      const validate = path.endsWith("/validate");
      const version = {
        ...document.current_version,
        ...(!validate
          ? {
              title: input.title,
              body_markdown: input.body_markdown,
              structured_content: input.structured_content,
            }
          : {}),
        public_id: crypto.randomUUID(),
        version: document.current_version.version + 1,
        status: validate ? ("validated" as const) : ("draft" as const),
        validated_at: validate ? now : null,
      };
      versions.unshift(version);
      document = {
        artifact: {
          ...document.artifact,
          title: version.title,
          status: version.status,
          current_version_id: version.public_id,
          version: version.version,
        },
        current_version: version,
      };
      return respond(document);
    }
    if (path.match(/^\/api\/artifacts\/[^/]+\/versions\/[^/]+$/))
      return respond(
        versions.find((version) => version.public_id === path.split("/")[5]),
      );
    if (path === `/api/artifacts/${document.artifact.public_id}/versions`)
      return respond({
        items: versions,
        total: versions.length,
        limit: 25,
        offset: 0,
      });
    if (path.match(/^\/api\/artifacts\/[^/]+\/versions\/[^/]+\/export$/)) {
      const version = versions.find((v) => v.public_id === path.split("/")[5]);
      if (url.searchParams.get("format") === "markdown") {
        await route.fulfill({
          contentType: "text/markdown",
          body: version?.body_markdown ?? "",
        });
        return true;
      }
      return respond(version);
    }
    if (path === `/api/artifacts/${document.artifact.public_id}/publications`) {
      if (method === "POST") {
        const input = body();
        const publication = {
          ...fixturePublication,
          public_id: crypto.randomUUID(),
          artifact_id: document.artifact.public_id,
          version_id: input.version_id,
          connection_id: input.connection_id,
          provider: input.expected_provider,
          target_id: input.expected_target_id,
        };
        publications.push(publication);
        return respond(publication);
      }
      return respond({ items: publications, limit: 10, offset: 0 });
    }
    if (path.match(/^\/api\/publications\/[^/]+/)) {
      const item = publications.find((p) => p.public_id === path.split("/")[3]);
      if (!item) return respond({ message: "Publication inconnue" }, 404);
      if (path.endsWith("/cancel")) {
        item.status = "cancelled";
        return respond(item);
      }
      return respond({ publication: item, observations: [] });
    }
    if (path === `/api/projects/${project.public_id}/code-observations`) {
      if (method === "POST") {
        code = { ...code, corpus: { ...code.corpus, ...body() } };
        return respond(code);
      }
      return respond({ items: [code.corpus], limit: 25, offset: 0 });
    }
    if (path === `/api/code-observations/${code.corpus.public_id}`)
      return respond(code);
    if (
      path === "/api/company/graph" ||
      path === `/api/projects/${project.public_id}/graph`
    )
      return respond({
        workspace_public_id: workspaceId,
        project_public_id: path.includes("/projects/")
          ? project.public_id
          : null,
        nodes: [],
        edges: [],
        source_graph_versions: [],
        truncated: false,
        limits: { max_nodes: 500, max_edges: 1000 },
      });
    return false;
  };
}
