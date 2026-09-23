import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, FileText, Plus, Search, Settings2 } from "lucide-react";
import { useRef, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";
import { api, ApiError, createIdempotencyKey } from "@/api/client";
import { companyApi } from "@/api/company";
import { artifactsApi } from "@/api/artifacts";
import { artifactTypes } from "@/api/artifact-types";
import type { ArtifactContent, CreateArtifact } from "@/api/artifact-types";
import { ArtifactGenerator } from "@/components/artifacts/artifact-generator";
import { ArtifactEditor } from "@/components/artifacts/artifact-editor";
import { artifactLabels } from "@/components/artifacts/labels";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  NotFoundState,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatDate } from "@/lib/format";

const PAGE_SIZE = 25;
export function ArtifactsPage() {
  const route = useParams();
  const [params, setParams] = useSearchParams();
  const navigate = useNavigate();
  const cache = useQueryClient();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const requestedProjectId = route.projectId ?? params.get("project");
  const projectId =
    requestedProjectId === company.data?.company_scope?.project_public_id
      ? null
      : requestedProjectId;
  const activeProject = company.data?.projects.find(
    (item) => item.public_id === projectId,
  );
  const archived = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId!),
    enabled: Boolean(projectId && company.data && !activeProject),
  });
  const project =
    activeProject ??
    (archived.data?.project.public_id === projectId
      ? archived.data.project
      : undefined);
  const isArchived = project?.status === "archived";
  const scopeId = projectId
    ? project?.public_id
    : company.data?.company_scope?.project_public_id;
  const [search, setSearch] = useState(params.get("q") ?? "");
  const type =
    artifactTypes.find((item) => item === params.get("draft_type")) ??
    "specification";
  const filterType = artifactTypes.find((item) => item === params.get("type"));
  const status =
    params.get("status") === "validated"
      ? "validated"
      : params.get("status") === "draft"
        ? "draft"
        : undefined;
  const offset = Math.max(0, Number(params.get("offset")) || 0);
  const creating = params.get("create") === "1";
  const sourceSessionId = params.get("session");
  const documents = useQuery({
    queryKey: [
      "artifacts",
      scopeId,
      params.get("q") ?? "",
      filterType,
      status,
      offset,
    ],
    queryFn: () =>
      artifactsApi.list(scopeId!, {
        q: params.get("q") ?? undefined,
        type: filterType,
        status,
        limit: PAGE_SIZE,
        offset,
      }),
    enabled: Boolean(scopeId),
  });
  const snapshot = useQuery({
    queryKey: ["snapshot", scopeId],
    queryFn: () => api.snapshot(scopeId!),
    enabled: Boolean(scopeId && creating),
  });
  const conversation = useQuery({
    queryKey: ["session", scopeId, sourceSessionId],
    queryFn: () => api.session(scopeId!, sourceSessionId!),
    enabled: Boolean(scopeId && creating && sourceSessionId),
  });
  const previous = useRef<{ payload: string; key: string } | null>(null);
  const create = useMutation({
    mutationFn: (input: CreateArtifact) => {
      const payload = JSON.stringify({ scopeId, input });
      if (previous.current?.payload !== payload)
        previous.current = { payload, key: createIdempotencyKey() };
      return artifactsApi.create(scopeId!, input, previous.current.key);
    },
    onSuccess: (detail) => {
      void cache.invalidateQueries({ queryKey: ["artifacts", scopeId] });
      cache.setQueryData(["artifact", detail.artifact.public_id], detail);
      navigate(`/artifacts/${detail.artifact.public_id}`);
    },
  });
  function updateFilters(values: Record<string, string>) {
    const next = new URLSearchParams(params);
    for (const [key, value] of Object.entries(values)) {
      if (value) next.set(key, value);
      else next.delete(key);
    }
    setParams(next);
  }
  if (company.isPending)
    return <LoadingState label="Chargement de la bibliothèque…" />;
  if (company.isError)
    return (
      <ErrorState error={company.error} retry={() => void company.refetch()} />
    );
  if (projectId && !activeProject && archived.isPending)
    return <LoadingState label="Chargement du projet…" />;
  if (
    projectId &&
    !activeProject &&
    archived.isError &&
    !(archived.error instanceof ApiError && archived.error.status === 404)
  )
    return (
      <ErrorState
        error={archived.error}
        retry={() => void archived.refetch()}
      />
    );
  if (projectId && !project)
    return <NotFoundState title="Projet inaccessible" />;
  if (!scopeId)
    return (
      <EmptyState
        title="Configurez votre entreprise"
        description="Son contexte général accueillera vos premiers livrables."
        action={
          <Button asChild>
            <Link to="/company">Configurer l’entreprise</Link>
          </Button>
        }
      />
    );
  const canEdit = company.data.workspace.role !== "viewer" && !isArchived;
  const initial: ArtifactContent = {
    title: "",
    body_markdown: "",
    structured_content: {},
    sources: conversation.data
      ? [{ kind: "session", public_id: conversation.data.session.public_id }]
      : [],
  };
  return (
    <div className="space-y-7">
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-start">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Livrables</h1>
          <p className="mt-2 text-muted-foreground">
            Des brouillons aux versions validées, conservez le contenu et ses
            sources.
          </p>
        </div>
        {canEdit && !creating ? (
          <Button onClick={() => updateFilters({ create: "1" })}>
            <Plus /> Nouveau livrable
          </Button>
        ) : null}
      </header>
      <div className="flex flex-col justify-between gap-4 border-b pb-5 sm:flex-row sm:items-end">
        <div className="w-full max-w-md">
          <label
            className="mb-2 block text-sm font-medium"
            htmlFor="artifact-scope"
          >
            Entreprise ou projet
          </label>
          <select
            id="artifact-scope"
            className="min-h-11 w-full rounded-md border bg-white px-3 text-sm"
            value={projectId ?? ""}
            disabled={create.isPending}
            onChange={(event) =>
              navigate(
                event.target.value
                  ? `/projects/${event.target.value}/artifacts`
                  : "/artifacts",
              )
            }
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
        {!isArchived ? (
          <Button asChild variant="outline">
            <Link
              to={
                projectId
                  ? `/settings/destinations?project=${projectId}`
                  : "/settings/destinations"
              }
            >
              <Settings2 /> Destinations
            </Link>
          </Button>
        ) : null}
      </div>
      {isArchived ? (
        <p role="status" className="rounded-md bg-muted p-4 text-sm">
          Projet archivé : ses livrables et leurs versions restent consultables
          en lecture seule.
        </p>
      ) : null}
      {creating && canEdit ? (
        <section className="space-y-5 rounded-lg border p-5 sm:p-7">
          <div className="flex items-center justify-between gap-3">
            <h2 className="text-xl font-semibold">Préparer un brouillon</h2>
            <Button
              variant="ghost"
              disabled={create.isPending}
              onClick={() => updateFilters({ create: "", session: "" })}
            >
              Fermer
            </Button>
          </div>
          <div className="max-w-md">
            <label
              htmlFor="artifact-type"
              className="mb-2 block text-sm font-medium"
            >
              Type de livrable
            </label>
            <select
              id="artifact-type"
              value={type}
              onChange={(event) =>
                updateFilters({ draft_type: event.target.value })
              }
              disabled={create.isPending}
              className="min-h-11 w-full rounded-md border bg-white px-3 text-sm"
            >
              {artifactTypes.map((item) => (
                <option key={item} value={item}>
                  {artifactLabels[item]}
                </option>
              ))}
            </select>
          </div>
          {snapshot.isPending || (sourceSessionId && conversation.isPending) ? (
            <LoadingState label="Chargement des sources…" />
          ) : snapshot.isError || conversation.isError ? (
            <ErrorState
              error={(snapshot.error ?? conversation.error)!}
              retry={() => {
                void snapshot.refetch();
                if (sourceSessionId) void conversation.refetch();
              }}
            />
          ) : (
            <>
              <ArtifactGenerator
                key={`${scopeId}:${type}:${sourceSessionId ?? ""}`}
                projectId={scopeId}
                type={type}
                sessions={snapshot.data?.sessions ?? []}
                selectedSessionId={sourceSessionId ?? undefined}
              />
              <ArtifactEditor
                key={`${scopeId}:${sourceSessionId ?? "manual"}`}
                initial={initial}
                snapshot={snapshot.data}
                conversation={conversation.data}
                busy={create.isPending}
                submitLabel="Créer le brouillon"
                onSubmit={(content) =>
                  create.mutate({ ...content, artifact_type: type })
                }
              />
            </>
          )}
          {create.error ? (
            <ErrorState
              title="Le brouillon n’a pas pu être confirmé"
              error={create.error}
            />
          ) : null}
        </section>
      ) : null}
      <form
        className="flex flex-col gap-3 xl:flex-row"
        onSubmit={(event) => {
          event.preventDefault();
          updateFilters({ q: search.trim(), offset: "" });
        }}
      >
        <label htmlFor="artifact-search" className="sr-only">
          Rechercher dans les livrables
        </label>
        <Input
          id="artifact-search"
          className="sm:max-w-md"
          value={search}
          maxLength={200}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Rechercher un titre ou un contenu…"
        />
        <Button type="submit" variant="outline">
          <Search /> Rechercher
        </Button>
        <select
          aria-label="Filtrer par type"
          value={filterType ?? ""}
          className="min-h-10 rounded-md border bg-white px-3 text-sm"
          onChange={(event) =>
            updateFilters({ type: event.target.value, offset: "" })
          }
        >
          <option value="">Tous les types</option>
          {artifactTypes.map((item) => (
            <option key={item} value={item}>
              {artifactLabels[item]}
            </option>
          ))}
        </select>
        <select
          aria-label="Filtrer par état"
          value={status ?? ""}
          className="min-h-10 rounded-md border bg-white px-3 text-sm"
          onChange={(event) =>
            updateFilters({ status: event.target.value, offset: "" })
          }
        >
          <option value="">Tous les états</option>
          <option value="draft">Brouillons</option>
          <option value="validated">Validés</option>
        </select>
      </form>
      {documents.isPending ? (
        <LoadingState label="Chargement des livrables…" />
      ) : documents.isError ? (
        <ErrorState
          error={documents.error}
          retry={() => void documents.refetch()}
        />
      ) : (
        <>
          {documents.data.items.length ? (
            <div className="divide-y rounded-lg border">
              {documents.data.items.map((item) => (
                <Link
                  key={item.public_id}
                  to={`/artifacts/${item.public_id}`}
                  className="flex items-start justify-between gap-4 p-5 hover:bg-muted/40"
                >
                  <div className="flex min-w-0 gap-4">
                    <FileText className="mt-1 size-5 shrink-0 text-primary" />
                    <div className="min-w-0">
                      <h2 className="break-words font-semibold">
                        {item.title}
                      </h2>
                      <p className="mt-1 text-sm text-muted-foreground">
                        {artifactLabels[item.artifact_type]} · version{" "}
                        {item.version} ·{" "}
                        {item.status === "validated" ? "Validé" : "Brouillon"}
                      </p>
                      <p className="mt-2 text-xs text-muted-foreground">
                        Mis à jour {formatDate(item.updated_at)}
                      </p>
                    </div>
                  </div>
                  <ArrowRight className="size-4 shrink-0 text-muted-foreground" />
                </Link>
              ))}
            </div>
          ) : (
            <EmptyState
              title="Aucun livrable trouvé"
              description={
                params.get("q") || filterType || status
                  ? "Ajustez la recherche ou les filtres pour retrouver un autre document."
                  : "Préparez un premier brouillon à partir de vos connaissances ou d’une conversation."
              }
            />
          )}
          <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
            <p className="text-muted-foreground">
              {documents.data.total
                ? `${offset + 1}–${offset + documents.data.items.length} sur ${documents.data.total}`
                : "0 livrable"}
            </p>
            <div className="flex gap-2">
              <Button
                variant="outline"
                disabled={offset === 0}
                onClick={() =>
                  updateFilters({
                    offset: String(Math.max(0, offset - PAGE_SIZE)),
                  })
                }
              >
                Précédent
              </Button>
              <Button
                variant="outline"
                disabled={
                  offset + documents.data.items.length >= documents.data.total
                }
                onClick={() =>
                  updateFilters({ offset: String(offset + PAGE_SIZE) })
                }
              >
                Suivant
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
