import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Check, Pencil } from "lucide-react";
import { useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import { api, createIdempotencyKey } from "@/api/client";
import { artifactsApi } from "@/api/artifacts";
import { companyApi } from "@/api/company";
import type {
  ArtifactDetail,
  ArtifactVersion,
  SaveArtifact,
} from "@/api/artifact-types";
import { ArtifactEditor } from "@/components/artifacts/artifact-editor";
import { ArtifactExport } from "@/components/artifacts/artifact-export";
import { ArtifactPublications } from "@/components/work-tools/artifact-publications";
import { VersionComparison } from "@/components/artifacts/version-comparison";
import { artifactLabels } from "@/components/artifacts/labels";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { formatDate } from "@/lib/format";

export function ArtifactPage() {
  const { artifactId = "" } = useParams();
  return <ArtifactView key={artifactId} artifactId={artifactId} />;
}

function ArtifactView({ artifactId }: { artifactId: string }) {
  const [params, setParams] = useSearchParams();
  const cache = useQueryClient();
  const detail = useQuery({
    queryKey: ["artifact", artifactId],
    queryFn: () => artifactsApi.detail(artifactId),
  });
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const [offset, setOffset] = useState(0);
  const history = useQuery({
    queryKey: ["artifact-versions", artifactId, offset],
    queryFn: () => artifactsApi.versions(artifactId, { limit: 25, offset }),
  });
  const selectedId =
    params.get("version") ?? detail.data?.current_version.public_id;
  const comparisonId = params.get("compare");
  const selectedQuery = useQuery({
    queryKey: ["artifact-version", artifactId, selectedId],
    queryFn: () => artifactsApi.version(artifactId, selectedId!),
    enabled: Boolean(
      selectedId && selectedId !== detail.data?.current_version.public_id,
    ),
  });
  const comparison = useQuery({
    queryKey: ["artifact-version", artifactId, comparisonId],
    queryFn: () => artifactsApi.version(artifactId, comparisonId!),
    enabled: Boolean(comparisonId),
  });
  const [editingVersion, setEditingVersion] = useState<ArtifactVersion | null>(
    null,
  );
  const snapshot = useQuery({
    queryKey: ["snapshot", detail.data?.artifact.project_id],
    queryFn: () => api.snapshot(detail.data!.artifact.project_id),
    enabled: Boolean(detail.data && editingVersion),
  });
  const previous = useRef<{ payload: string; key: string } | null>(null);
  const validationCommand = useRef<{ versionId: string; key: string } | null>(
    null,
  );
  function saved(value: ArtifactDetail) {
    cache.setQueryData(["artifact", artifactId], value);
    void cache.invalidateQueries({
      queryKey: ["artifact-versions", artifactId],
    });
    void cache.invalidateQueries({
      queryKey: ["artifacts", value.artifact.project_id],
    });
    setEditingVersion(null);
    setParams({});
  }
  const save = useMutation({
    mutationFn: (input: SaveArtifact) => {
      const payload = JSON.stringify(input);
      if (previous.current?.payload !== payload)
        previous.current = { payload, key: createIdempotencyKey() };
      return artifactsApi.save(artifactId, input, previous.current.key);
    },
    onSuccess: saved,
  });
  const validate = useMutation({
    mutationFn: (versionId: string) => {
      if (validationCommand.current?.versionId !== versionId)
        validationCommand.current = { versionId, key: createIdempotencyKey() };
      return artifactsApi.validate(
        artifactId,
        versionId,
        validationCommand.current.key,
      );
    },
    onSuccess: saved,
  });
  if (detail.isPending) return <LoadingState label="Chargement du livrable…" />;
  if (detail.isError)
    return (
      <ErrorState error={detail.error} retry={() => void detail.refetch()} />
    );
  const { artifact, current_version: current } = detail.data;
  const historical = selectedId !== current.public_id;
  const selected = historical ? selectedQuery.data : current;
  const canEdit =
    (company.data?.workspace.role === "owner" ||
      company.data?.workspace.role === "editor") &&
    (company.data?.company_scope?.project_public_id === artifact.project_id ||
      company.data?.projects.some(
        (project) =>
          project.public_id === artifact.project_id &&
          project.status === "active",
      ) === true);
  const companyScope =
    company.data?.company_scope?.project_public_id === artifact.project_id;
  const project = company.data?.projects.find(
    (item) => item.public_id === artifact.project_id,
  );
  const libraryUrl = companyScope
    ? "/artifacts"
    : `/projects/${artifact.project_id}/artifacts`;
  const busy = save.isPending || validate.isPending;
  const versions = new Map(
    (history.data?.items ?? []).map((version) => [version.public_id, version]),
  );
  versions.set(current.public_id, current);
  if (selected) versions.set(selected.public_id, selected);
  if (comparison.data) versions.set(comparison.data.public_id, comparison.data);
  function changeVersion(value: string) {
    setEditingVersion(null);
    save.reset();
    validate.reset();
    setParams(value === current.public_id ? {} : { version: value });
  }
  return (
    <div className="space-y-7">
      <nav
        aria-label="Contexte du livrable"
        className="flex flex-wrap items-center gap-2 text-sm"
      >
        <Button asChild variant="ghost">
          <Link
            to={companyScope ? "/company" : `/projects/${artifact.project_id}`}
          >
            {companyScope
              ? "Contexte de l’entreprise"
              : (project?.name ?? "Ouvrir le projet")}
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link to={libraryUrl}>
            <ArrowLeft /> Bibliothèque des livrables
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link
            to={
              companyScope ? "/graph" : `/graph?project=${artifact.project_id}`
            }
          >
            {companyScope ? "Graphe de l’entreprise" : "Graphe du projet"}
          </Link>
        </Button>
      </nav>
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-start">
        <div>
          <p className="text-sm text-muted-foreground">
            {artifactLabels[artifact.artifact_type]}
          </p>
          <h1 className="mt-2 break-words text-3xl font-semibold tracking-tight">
            {selected?.title ?? artifact.title}
          </h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {selected
              ? `Version ${selected.version} · ${selected.status === "validated" ? "Validée" : "Brouillon"} · ${formatDate(selected.created_at, true)}`
              : "Chargement de la version…"}
          </p>
        </div>
        {!historical && !editingVersion && canEdit ? (
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              disabled={busy}
              onClick={() => {
                save.reset();
                setEditingVersion(current);
              }}
            >
              <Pencil /> Nouvelle révision
            </Button>
            {current.status === "draft" ? (
              <Button
                disabled={busy}
                onClick={() => validate.mutate(current.public_id)}
              >
                <Check />{" "}
                {validate.isPending ? "Validation…" : "Valider cette version"}
              </Button>
            ) : null}
          </div>
        ) : null}
      </header>
      {validate.error ? (
        <ErrorState
          title="La validation n’a pas pu être confirmée"
          error={validate.error}
          retry={() => validate.mutate(validate.variables ?? current.public_id)}
        />
      ) : null}
      {historical ? (
        <p className="rounded-md border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Vous consultez une version historique. La version courante est la{" "}
          {current.version}.{" "}
          <button
            className="font-medium underline"
            onClick={() => changeVersion(current.public_id)}
          >
            Ouvrir la version courante
          </button>
        </p>
      ) : null}
      <section
        className="flex flex-wrap items-end gap-4 border-y py-5"
        aria-label="Historique des versions"
      >
        <div>
          <label
            htmlFor="artifact-version"
            className="mb-2 block text-sm font-medium"
          >
            Version affichée
          </label>
          <select
            id="artifact-version"
            disabled={busy}
            value={selectedId ?? ""}
            onChange={(event) => changeVersion(event.target.value)}
            className="min-h-10 rounded-md border bg-white px-3 text-sm"
          >
            {Array.from(versions.values())
              .sort((a, b) => b.version - a.version)
              .map((version) => (
                <option key={version.public_id} value={version.public_id}>
                  Version {version.version} ·{" "}
                  {version.status === "validated" ? "validée" : "brouillon"}
                </option>
              ))}
          </select>
        </div>
        <div>
          <label
            htmlFor="artifact-compare"
            className="mb-2 block text-sm font-medium"
          >
            Comparer avec
          </label>
          <select
            id="artifact-compare"
            disabled={busy || Boolean(editingVersion)}
            value={comparisonId ?? ""}
            onChange={(event) => {
              const next = new URLSearchParams(params);
              if (event.target.value) next.set("compare", event.target.value);
              else next.delete("compare");
              setParams(next);
            }}
            className="min-h-10 rounded-md border bg-white px-3 text-sm"
          >
            <option value="">Sans comparaison</option>
            {Array.from(versions.values())
              .filter((version) => version.public_id !== selectedId)
              .map((version) => (
                <option key={version.public_id} value={version.public_id}>
                  Version {version.version}
                </option>
              ))}
          </select>
        </div>
        {history.data ? (
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span>{history.data.total} version(s)</span>
            <Button
              variant="ghost"
              size="sm"
              disabled={offset === 0 || busy}
              onClick={() => setOffset(Math.max(0, offset - 25))}
            >
              Plus récentes
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={
                offset + history.data.items.length >= history.data.total || busy
              }
              onClick={() => setOffset(offset + 25)}
            >
              Plus anciennes
            </Button>
          </div>
        ) : null}
      </section>
      {history.error ? (
        <ErrorState
          error={history.error}
          retry={() => void history.refetch()}
        />
      ) : null}
      {editingVersion && canEdit ? (
        <section className="space-y-5 rounded-lg border p-6">
          <div className="flex items-center justify-between gap-4">
            <h2 className="text-xl font-semibold">
              Préparer une nouvelle révision
            </h2>
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => setEditingVersion(null)}
            >
              Annuler l’édition
            </Button>
          </div>
          <p className="text-sm text-muted-foreground">
            La version {editingVersion.version} restera conservée.
            L’enregistrement créera un nouveau brouillon.
          </p>
          {snapshot.isPending ? (
            <LoadingState label="Chargement des sources…" />
          ) : snapshot.isError ? (
            <ErrorState
              error={snapshot.error}
              retry={() => void snapshot.refetch()}
            />
          ) : (
            <ArtifactEditor
              key={editingVersion.public_id}
              initial={editingVersion}
              sourceSnapshots={editingVersion.sources}
              snapshot={snapshot.data}
              busy={busy}
              submitLabel="Enregistrer la nouvelle version"
              onSubmit={(content) =>
                save.mutate({
                  ...content,
                  expected_version_id: editingVersion.public_id,
                })
              }
            />
          )}
          {save.error ? (
            <ErrorState
              title="La révision n’a pas pu être confirmée ; votre saisie reste disponible"
              error={save.error}
            />
          ) : null}
        </section>
      ) : historical && selectedQuery.isError ? (
        <ErrorState
          error={selectedQuery.error}
          retry={() => void selectedQuery.refetch()}
        />
      ) : !selected ? (
        <LoadingState label="Chargement de la version sélectionnée…" />
      ) : comparisonId ? (
        comparison.isPending ? (
          <LoadingState label="Chargement de la comparaison…" />
        ) : comparison.isError ? (
          <ErrorState
            error={comparison.error}
            retry={() => void comparison.refetch()}
          />
        ) : (
          <VersionComparison before={comparison.data} after={selected} />
        )
      ) : (
        <section className="grid gap-6 lg:grid-cols-[minmax(0,1.6fr)_minmax(250px,0.7fr)]">
          <article className="min-w-0 rounded-lg border p-6">
            <h2 className="mb-5 text-lg font-semibold">
              Contenu de la version {selected.version}
            </h2>
            <div className="whitespace-pre-wrap break-words text-sm leading-7">
              {selected.body_markdown ||
                "Ce brouillon ne contient pas encore de texte."}
            </div>
            {Object.keys(selected.structured_content).length ? (
              <details className="mt-6 border-t pt-4 text-sm">
                <summary>Données structurées conservées</summary>
                <pre className="mt-3 overflow-x-auto whitespace-pre-wrap break-words text-xs">
                  {JSON.stringify(selected.structured_content, null, 2)}
                </pre>
              </details>
            ) : null}
          </article>
          <aside className="h-fit space-y-5 rounded-lg border p-6">
            <h2 className="font-semibold">Sources de cette version</h2>
            {selected.sources.length ? (
              <ul className="space-y-4">
                {selected.sources.map((source) => (
                  <li
                    key={`${source.kind}:${source.public_id}`}
                    className="break-words text-sm"
                  >
                    <p className="font-medium">{source.title || source.kind}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {source.version ? `Version ${source.version} · ` : ""}
                      {source.status_at_capture ?? "Source rattachée"}
                    </p>
                    <p className="mt-1 break-all text-xs text-muted-foreground">
                      {source.public_id}
                    </p>
                    {source.kind === "artifact_version" &&
                    source.artifact_id ? (
                      <Link
                        to={`/artifacts/${source.artifact_id}?version=${source.public_id}`}
                        className="mt-2 block text-sm text-primary"
                      >
                        Ouvrir cette version source
                      </Link>
                    ) : null}
                    {source.kind === "session" ? (
                      <Link
                        to={`/projects/${source.project_id}/sessions/${source.public_id}`}
                        className="mt-2 block text-sm text-primary"
                      >
                        Ouvrir la conversation
                      </Link>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-muted-foreground">
                Aucune source rattachée. Ce contenu a été rédigé manuellement.
              </p>
            )}
            <div className="border-t pt-4">
              <p className="text-xs font-medium">Version immuable</p>
              <p className="mt-1 break-all text-xs text-muted-foreground">
                {selected.public_id}
              </p>
              <p className="mt-3 text-xs font-medium">Empreinte du contenu</p>
              <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                {selected.content_hash}
              </p>
            </div>
          </aside>
        </section>
      )}
      {selected && !editingVersion ? (
        <section className="space-y-3 border-t pt-6">
          <h2 className="font-semibold">Emporter cette version</h2>
          <ArtifactExport
            key={selected.public_id}
            artifactId={artifactId}
            version={selected}
          />
          <p className="text-xs text-muted-foreground">
            L’export conserve l’identité, la version et les sources. Il ne
            constitue pas une publication dans un outil externe.
          </p>
        </section>
      ) : null}
      {selected && !editingVersion && company.data ? (
        <ArtifactPublications
          key={selected.public_id}
          artifactId={artifactId}
          version={selected}
          isCurrent={!historical}
          artifactType={artifact.artifact_type}
          projectId={companyScope ? undefined : artifact.project_id}
          canEdit={canEdit}
        />
      ) : null}
    </div>
  );
}
