import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Bot, History, Network, Plus } from "lucide-react";
import { useRef } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";

import { api, ApiError, createIdempotencyKey } from "@/api/client";
import { companyApi } from "@/api/company";
import type { CompanyAgent } from "@/api/company-types";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  NotFoundState,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { formatDate } from "@/lib/format";

const descriptions: Record<string, string> = {
  general:
    "Explorez les décisions et connaissances confirmées de votre entreprise.",
  product:
    "Précisez les besoins, les règles métier et les critères de réussite.",
  sales: "Préparez votre approche commerciale à partir du contexte partagé.",
  tech: "Préparez le plan technique à partir d’un contexte confirmé et transmis.",
  dev: "Clarifiez l’implémentation et préparez le travail dans votre outil de développement.",
};
const roleNames: Record<string, string> = {
  general: "Généraliste",
  product: "Produit",
  sales: "Commercial",
  tech: "Lead technique",
  dev: "Développement",
};

export function AgentsPage() {
  const [params, setParams] = useSearchParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const projectId = params.get("project");
  const requestedAgent = params.get("agent");
  const activeProject = company.data?.projects.find(
    (item) => item.public_id === projectId,
  );
  const scopeId = projectId || company.data?.company_scope?.project_public_id;
  const snapshot = useQuery({
    queryKey: ["snapshot", scopeId],
    queryFn: () => api.snapshot(scopeId!),
    enabled: Boolean(scopeId),
  });
  const project =
    activeProject ??
    (snapshot.data?.project.public_id === projectId
      ? snapshot.data.project
      : undefined);
  const isArchived = project?.status === "archived";
  const agents: CompanyAgent[] = isArchived
    ? (snapshot.data?.nodes.map((node) => ({
        node_public_id: node.public_id,
        project_public_id: project!.public_id,
        scope_kind: "project" as const,
        node_key: node.node_key,
        profile_key: node.profile_key,
        name: node.profile_name || node.title,
        role: node.title,
        requires_context_pack: ["tech", "dev"].includes(node.node_key),
      })) ?? [])
    : (company.data?.agents.filter(
        (agent) => agent.project_public_id === scopeId,
      ) ?? []);
  const selected =
    agents.find((agent) => agent.node_key === requestedAgent) ?? agents[0];
  const pendingCommand = useRef<{
    scopeId: string;
    nodeKey: string;
    key: string;
  } | null>(null);
  const create = useMutation({
    mutationFn: (agent: CompanyAgent) => {
      const previous = pendingCommand.current;
      const command =
        previous?.scopeId === agent.project_public_id &&
        previous.nodeKey === agent.node_key
          ? previous
          : {
              scopeId: agent.project_public_id,
              nodeKey: agent.node_key,
              key: createIdempotencyKey(),
            };
      pendingCommand.current = command;
      return api.createSession(
        agent.project_public_id,
        agent.node_key,
        undefined,
        command.key,
      );
    },
    onSuccess: (session, agent) => {
      pendingCommand.current = null;
      void queryClient.invalidateQueries({
        queryKey: ["snapshot", agent.project_public_id],
      });
      navigate(
        `/projects/${agent.project_public_id}/sessions/${session.session.public_id}`,
      );
    },
  });

  if (company.isPending)
    return <LoadingState label="Chargement des agents accessibles…" />;
  if (company.isError)
    return (
      <ErrorState error={company.error} retry={() => void company.refetch()} />
    );
  if (projectId && !activeProject && snapshot.isPending)
    return <LoadingState label="Chargement de l’historique du projet…" />;
  if (
    projectId &&
    !activeProject &&
    snapshot.isError &&
    !(snapshot.error instanceof ApiError && snapshot.error.status === 404)
  )
    return (
      <ErrorState
        error={snapshot.error}
        retry={() => void snapshot.refetch()}
      />
    );
  if (projectId && !project)
    return (
      <NotFoundState
        title="Projet inaccessible"
        description="Choisissez un projet auquel vous avez accès dans cette entreprise."
      />
    );
  const scopeName = project?.name ?? company.data.workspace.name;
  const sessions =
    snapshot.data?.sessions
      .filter((session) => session.node_key === selected?.node_key)
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at)) ?? [];
  const latest = sessions.find((session) => session.status === "active");
  const canCreate = company.data.workspace.role !== "viewer" && !isArchived;

  function changeScope(next: string) {
    create.reset();
    setParams(next ? { project: next } : {});
  }
  function changeAgent(nodeKey: string) {
    create.reset();
    setParams(
      projectId ? { project: projectId, agent: nodeKey } : { agent: nodeKey },
    );
  }

  return (
    <div className="space-y-7">
      <header>
        <h1 className="text-3xl font-semibold tracking-tight">
          Construire le projet, ensemble.
        </h1>
        <p className="mt-2 text-muted-foreground">
          Choisissez le contexte et le rôle avec lequel vous travaillez.
        </p>
      </header>
      <div className="flex flex-col gap-4 border-b pb-6 sm:flex-row sm:items-end sm:justify-between">
        <div className="w-full max-w-md">
          <label
            htmlFor="agent-scope"
            className="mb-2 block text-sm font-medium"
          >
            Contexte de la conversation
          </label>
          <select
            id="agent-scope"
            disabled={create.isPending}
            value={projectId ?? ""}
            onChange={(event) => changeScope(event.target.value)}
            className="min-h-11 w-full rounded-md border bg-white px-3 text-sm"
          >
            <option value="">Entreprise · {company.data.workspace.name}</option>
            {isArchived && project ? (
              <option value={project.public_id}>
                Projet archivé · {project.name}
              </option>
            ) : null}
            {company.data.projects.map((item) => (
              <option key={item.public_id} value={item.public_id}>
                Projet · {item.name}
              </option>
            ))}
          </select>
        </div>
        <Button asChild variant="outline">
          <Link to={projectId ? `/graph?project=${projectId}` : "/graph"}>
            <Network /> Voir le graphe
          </Link>
        </Button>
      </div>
      {isArchived ? (
        <p role="status" className="rounded-md bg-muted p-4 text-sm">
          Projet archivé : consultez ses conversations et leur contexte. Une
          restauration est nécessaire pour reprendre le travail.
        </p>
      ) : null}
      {!scopeId ? (
        <EmptyState
          title="Configurez le contexte de votre entreprise"
          description="L’espace général accueille les connaissances et conversations communes à l’équipe."
          action={
            <Button asChild>
              <Link to="/company">Configurer l’entreprise</Link>
            </Button>
          }
        />
      ) : !agents.length ? (
        <EmptyState
          title="Aucun agent disponible dans ce contexte"
          description="Vérifiez la configuration de votre entreprise pour activer ses profils de travail."
          action={
            <Button asChild variant="outline">
              <Link to="/company">Voir la configuration</Link>
            </Button>
          }
        />
      ) : (
        <>
          <div className="flex flex-wrap gap-2" aria-label="Rôle de l’agent">
            {agents.map((agent) => (
              <Button
                key={agent.node_public_id}
                variant={
                  agent.node_public_id === selected?.node_public_id
                    ? "default"
                    : "outline"
                }
                aria-pressed={agent.node_public_id === selected?.node_public_id}
                disabled={create.isPending}
                className="min-w-28"
                onClick={() => changeAgent(agent.node_key)}
              >
                {roleNames[agent.node_key] ?? agent.name}
              </Button>
            ))}
          </div>
          {selected ? (
            <div className="grid gap-6 lg:grid-cols-[minmax(0,1.55fr)_minmax(280px,1fr)]">
              <section
                className="overflow-hidden rounded-lg border"
                aria-labelledby="selected-agent-title"
              >
                <div className="flex items-start gap-4 border-b p-6">
                  <span className="grid size-12 shrink-0 place-items-center rounded-lg bg-indigo-50 text-primary">
                    <Bot aria-hidden className="size-6" />
                  </span>
                  <div>
                    <h2
                      id="selected-agent-title"
                      className="text-xl font-semibold"
                    >
                      {selected.name}
                    </h2>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {projectId ? "Projet" : "Entreprise"} · {scopeName}
                    </p>
                  </div>
                </div>
                <div className="space-y-5 p-6">
                  <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
                    {descriptions[selected.node_key] ?? selected.role}
                  </p>
                  {snapshot.isPending ? (
                    <LoadingState label="Recherche des conversations…" />
                  ) : snapshot.isError ? (
                    <ErrorState
                      error={snapshot.error}
                      retry={() => void snapshot.refetch()}
                    />
                  ) : (
                    <>
                      {latest ? (
                        <Button asChild>
                          <Link
                            to={`/projects/${scopeId}/sessions/${latest.public_id}`}
                          >
                            {isArchived
                              ? "Consulter la conversation"
                              : "Reprendre la conversation"}{" "}
                            <ArrowRight />
                          </Link>
                        </Button>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          Aucune conversation active avec cet agent dans ce
                          contexte.
                        </p>
                      )}
                      {canCreate ? (
                        selected.requires_context_pack ? (
                          <div className="rounded-md border border-amber-200 bg-amber-50 p-4 text-sm text-amber-950">
                            <p>
                              Le lead technique travaille à partir d’un contexte
                              confirmé. Préparez le relais depuis les décisions
                              produit.
                            </p>
                            <Button
                              asChild
                              variant="outline"
                              className="mt-4 bg-white"
                            >
                              <Link
                                to={`/projects/${scopeId}/handoff${selected.node_key === "dev" ? "?target=dev" : ""}`}
                              >
                                Préparer le relais technique <ArrowRight />
                              </Link>
                            </Button>
                          </div>
                        ) : (
                          <Button
                            variant={latest ? "outline" : "default"}
                            disabled={create.isPending}
                            onClick={() => create.mutate(selected)}
                          >
                            <Plus />{" "}
                            {create.isPending
                              ? "Ouverture…"
                              : "Nouvelle conversation"}
                          </Button>
                        )
                      ) : (
                        <p className="rounded-md bg-muted p-4 text-sm text-muted-foreground">
                          {isArchived
                            ? "L’archive conserve les conversations existantes en lecture seule."
                            : "Votre accès en lecture permet de consulter les conversations existantes. Un éditeur peut en ouvrir une nouvelle."}
                        </p>
                      )}
                    </>
                  )}
                  {create.error ? (
                    <ErrorState
                      title="La conversation n’a pas pu être ouverte"
                      error={create.error}
                      retry={() => create.mutate(selected)}
                    />
                  ) : null}
                </div>
                {sessions.length ? (
                  <div className="border-t">
                    <h3 className="flex items-center gap-2 px-6 pb-2 pt-5 text-sm font-medium">
                      <History className="size-4" /> Conversations de ce rôle
                    </h3>
                    <ul className="divide-y">
                      {sessions.map((session) => (
                        <li key={session.public_id}>
                          <Link
                            to={`/projects/${scopeId}/sessions/${session.public_id}`}
                            className="flex items-center justify-between gap-3 px-6 py-4 text-sm hover:bg-muted/50"
                          >
                            <span className="min-w-0">
                              <span className="block truncate font-medium">
                                {session.title}
                              </span>
                              <span className="mt-1 block text-xs text-muted-foreground">
                                {formatDate(session.updated_at, true)}
                              </span>
                            </span>
                            <ArrowRight
                              aria-hidden
                              className="size-4 shrink-0 text-muted-foreground"
                            />
                          </Link>
                        </li>
                      ))}
                    </ul>
                  </div>
                ) : null}
              </section>
              <aside className="h-fit space-y-6 rounded-lg border p-6">
                <h2 className="text-lg font-semibold">
                  Ce qui guide cette conversation
                </h2>
                <div className="space-y-3">
                  <p className="text-sm font-medium">
                    {projectId
                      ? "Objectif du projet"
                      : "Contexte de l’entreprise"}
                  </p>
                  <p className="text-sm leading-6 text-muted-foreground">
                    {project?.objective ||
                      company.data.workspace.description ||
                      "Les connaissances confirmées et leurs sources constituent le contexte de travail."}
                  </p>
                </div>
                <div className="border-t pt-5">
                  <p className="text-sm font-medium">Un périmètre explicite</p>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    Cette conversation reste rattachée à {scopeName}. Changer de
                    contexte ouvre ou reprend un autre échange, sans déplacer
                    son historique.
                  </p>
                </div>
                <div className="border-t pt-5">
                  <p className="text-sm font-medium">
                    Vous confirmez les décisions
                  </p>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    Les propositions de l’agent deviennent des connaissances
                    après votre confirmation. Le code est réalisé dans vos
                    outils de développement.
                  </p>
                </div>
                <Link
                  to={
                    projectId ? `/projects/${projectId}/deliverables` : "/graph"
                  }
                  className="flex items-center gap-2 text-sm font-medium text-primary"
                >
                  {projectId
                    ? "Voir les livrables du projet"
                    : "Explorer les sources de l’entreprise"}
                  <ArrowRight className="size-4" />
                </Link>
              </aside>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
