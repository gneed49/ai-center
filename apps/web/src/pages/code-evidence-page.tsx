import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import { api, createIdempotencyKey } from "@/api/client";
import { companyApi } from "@/api/company";
import { workToolsApi } from "@/api/work-tools";
import { codeObservationsApi } from "@/api/code-observations";
import type { CodeReadInput } from "@/api/code-observations";
import { codeReadValidation } from "@/lib/code-evidence";
import { formatDate } from "@/lib/format";
import { CodeObservationView } from "@/components/code-evidence/observation-view";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  NotFoundState,
  PageHeader,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
export function CodeEvidencePage() {
  const { projectId = "" } = useParams();
  return <ProjectCodeEvidence key={projectId} projectId={projectId} />;
}
function ProjectCodeEvidence({ projectId }: { projectId: string }) {
  const [params, setParams] = useSearchParams();
  const cache = useQueryClient();
  const [offset, setOffset] = useState(0);
  const [creating, setCreating] = useState(false);
  const [repository, setRepository] = useState("");
  const [commit, setCommit] = useState("");
  const [pathsText, setPathsText] = useState("");
  const [connectionId, setConnectionId] = useState("");
  const [validation, setValidation] = useState<string | null>(null);
  const previous = useRef<{ fingerprint: string; key: string } | null>(null);
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
  });
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const tools = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const list = useQuery({
    queryKey: ["code-observations", projectId, offset],
    queryFn: () => codeObservationsApi.list(projectId, offset),
  });
  const selectedId =
    params.get("observation") ?? list.data?.items[0]?.public_id;
  const detail = useQuery({
    queryKey: ["code-observation", selectedId],
    queryFn: () => codeObservationsApi.detail(selectedId!),
    enabled: Boolean(selectedId),
  });
  const read = useMutation({
    mutationFn: (input: CodeReadInput) => {
      const fingerprint = JSON.stringify([projectId, input]);
      if (previous.current?.fingerprint !== fingerprint)
        previous.current = { fingerprint, key: createIdempotencyKey() };
      return codeObservationsApi.read(projectId, input, previous.current.key);
    },
    onSuccess: (result) => {
      previous.current = null;
      cache.setQueryData(["code-observation", result.corpus.public_id], result);
      void cache.invalidateQueries({
        queryKey: ["code-observations", projectId],
      });
      setParams({ observation: result.corpus.public_id });
      setCreating(false);
    },
  });
  const connections =
    tools.data?.connections.filter(
      (connection) => connection.provider === "github" && connection.enabled,
    ) ?? [];
  const selected =
    connections.find((connection) => connection.public_id === connectionId) ??
    connections[0];
  const paths = pathsText
    .split("\n")
    .map((path) => path.trim())
    .filter(Boolean);
  if (snapshot.isPending || company.isPending)
    return <LoadingState label="Chargement des preuves de code…" />;
  if (snapshot.isError || company.isError)
    return (
      <ErrorState
        error={(snapshot.error ?? company.error)!}
        retry={() => {
          void snapshot.refetch();
          void company.refetch();
        }}
      />
    );
  const canRead =
    company.data.workspace.role !== "viewer" &&
    snapshot.data.project.status !== "archived";
  return (
    <div className="space-y-7">
      <PageHeader
        title="Preuves de code"
        eyebrow={snapshot.data.project.name}
        description="Conservez une lecture ciblée de fichiers GitHub à un commit précis. Chaque observation reste consultable avec sa provenance."
        actions={
          <Button asChild variant="outline">
            <Link to={`/projects/${projectId}`}>Retour au projet</Link>
          </Button>
        }
      />
      {canRead ? (
        <Button
          disabled={read.isPending}
          onClick={() => setCreating((value) => !value)}
        >
          {creating ? "Fermer le formulaire" : "Observer des fichiers GitHub"}
        </Button>
      ) : (
        <p className="text-sm text-muted-foreground">
          Les observations existantes restent accessibles. Une nouvelle lecture
          nécessite un projet actif et un accès collaborateur.
        </p>
      )}
      {creating && canRead ? (
        <form
          className="space-y-5 rounded-lg border p-6"
          onSubmit={(event) => {
            event.preventDefault();
            const error = codeReadValidation(
              repository.trim(),
              commit.trim(),
              paths,
            );
            setValidation(error);
            if (!error && selected)
              read.mutate({
                connection_id: selected.public_id,
                repository: repository.trim(),
                commit_sha: commit.trim(),
                paths,
              });
          }}
        >
          <h2 className="text-lg font-semibold">Choisir ce qui sera lu</h2>
          {tools.isPending ? (
            <LoadingState label="Chargement des connexions GitHub…" />
          ) : tools.isError ? (
            <ErrorState
              error={tools.error}
              retry={() => void tools.refetch()}
            />
          ) : !selected ? (
            <p className="text-sm text-muted-foreground">
              Ajoutez une connexion GitHub autorisée dans les{" "}
              <Link to="/settings/tools" className="text-primary underline">
                outils de l’équipe
              </Link>
              .
            </p>
          ) : (
            <>
              <div>
                <label
                  htmlFor="code-connection"
                  className="mb-2 block text-sm font-medium"
                >
                  Connexion GitHub
                </label>
                <select
                  id="code-connection"
                  className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
                  value={selected.public_id}
                  disabled={read.isPending}
                  onChange={(event) => setConnectionId(event.target.value)}
                >
                  {connections.map((connection) => (
                    <option
                      key={connection.public_id}
                      value={connection.public_id}
                    >
                      {connection.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="grid gap-4 lg:grid-cols-2">
                <div>
                  <label
                    htmlFor="code-repository"
                    className="mb-2 block text-sm font-medium"
                  >
                    Dépôt (organisation/dépôt)
                  </label>
                  <Input
                    id="code-repository"
                    value={repository}
                    required
                    disabled={read.isPending}
                    onChange={(event) => setRepository(event.target.value)}
                    placeholder="organisation/projet"
                  />
                </div>
                <div>
                  <label
                    htmlFor="code-commit"
                    className="mb-2 block text-sm font-medium"
                  >
                    Commit exact
                  </label>
                  <Input
                    id="code-commit"
                    value={commit}
                    required
                    maxLength={40}
                    disabled={read.isPending}
                    onChange={(event) => setCommit(event.target.value)}
                    placeholder="Identifiant complet de 40 caractères"
                  />
                </div>
              </div>
              <div>
                <label
                  htmlFor="code-paths"
                  className="mb-2 block text-sm font-medium"
                >
                  Fichiers à observer, un chemin par ligne
                </label>
                <Textarea
                  id="code-paths"
                  value={pathsText}
                  rows={5}
                  required
                  disabled={read.isPending}
                  onChange={(event) => setPathsText(event.target.value)}
                  placeholder="src/application.ts"
                />
              </div>
              <p className="text-sm leading-6 text-muted-foreground">
                Cette action contacte GitHub pour lire au plus dix fichiers, 64
                Kio par fichier et 256 Kio au total. Les fichiers manquants,
                binaires ou indisponibles restent signalés. Aucun code n’est
                exécuté.
              </p>
              {!tools.data.storage_available ? (
                <p role="status" className="text-sm text-amber-900">
                  Le stockage sécurisé des connexions est indisponible. Une
                  nouvelle lecture sera possible après sa configuration.
                </p>
              ) : null}
              <Button
                type="submit"
                disabled={read.isPending || !tools.data.storage_available}
              >
                {read.isPending
                  ? "Lecture des fichiers sélectionnés…"
                  : "Lire et conserver cette observation"}
              </Button>
            </>
          )}
          {validation ? (
            <p role="alert" className="text-sm text-destructive">
              {validation}
            </p>
          ) : null}
          {read.error ? (
            <ErrorState
              title="La lecture n’a pas pu être confirmée"
              error={read.error}
            />
          ) : null}
        </form>
      ) : null}
      {list.isPending ? (
        <LoadingState label="Chargement des observations…" />
      ) : list.isError ? (
        <ErrorState error={list.error} retry={() => void list.refetch()} />
      ) : (
        <>
          <div className="flex flex-wrap items-end justify-between gap-4">
            <div className="w-full max-w-xl">
              <label
                htmlFor="code-corpus"
                className="mb-2 block text-sm font-medium"
              >
                Observation conservée
              </label>
              <select
                id="code-corpus"
                className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
                value={selectedId ?? ""}
                onChange={(event) =>
                  setParams({ observation: event.target.value })
                }
              >
                {!list.data.items.length ? (
                  <option value="">Aucune observation</option>
                ) : null}
                {selectedId &&
                !list.data.items.some(
                  (item) => item.public_id === selectedId,
                ) ? (
                  <option value={selectedId}>Observation sélectionnée</option>
                ) : null}
                {list.data.items.map((corpus) => (
                  <option key={corpus.public_id} value={corpus.public_id}>
                    {corpus.repository} · {corpus.commit_sha.slice(0, 10)} ·{" "}
                    {formatDate(corpus.observed_at, true)}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex gap-2">
              <Button
                variant="outline"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 25))}
              >
                Plus récentes
              </Button>
              <Button
                variant="outline"
                disabled={list.data.items.length < 25}
                onClick={() => setOffset(offset + 25)}
              >
                Plus anciennes
              </Button>
            </div>
          </div>
          {selectedId ? (
            detail.isPending ? (
              <LoadingState label="Chargement du contenu observé…" />
            ) : detail.isError ? (
              <ErrorState
                error={detail.error}
                retry={() => void detail.refetch()}
              />
            ) : detail.data.corpus.project_id !== projectId ? (
              <NotFoundState title="Observation hors de ce projet" />
            ) : (
              <CodeObservationView
                key={`${detail.data.corpus.public_id}:${params.get("file") ?? ""}`}
                detail={detail.data}
                initialFileId={params.get("file") ?? undefined}
                onSelectFile={(fileId) =>
                  setParams({
                    observation: detail.data.corpus.public_id,
                    file: fileId,
                  })
                }
              />
            )
          ) : (
            <EmptyState
              title="Aucune observation de code"
              description="Choisissez quelques fichiers utiles à une question précise pour garder une preuve de ce qui a été lu."
            />
          )}
        </>
      )}
    </div>
  );
}
