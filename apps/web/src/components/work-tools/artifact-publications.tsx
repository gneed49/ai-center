import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Link } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { workToolsApi } from "@/api/work-tools";
import { createIdempotencyKey } from "@/api/client";
import type { ArtifactType, ArtifactVersion } from "@/api/artifact-types";
import type { PublishArtifact, WorkToolProvider } from "@/api/work-tool-types";
import {
  publicationLabels,
  safeWorkToolUrl,
} from "@/lib/work-tool-publication";
import { formatDate } from "@/lib/format";
import { providerLabels } from "@/components/artifacts/labels";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, LoadingState } from "@/components/app/page";
import { PublicationDetailPanel } from "./publication-detail";

type Review = {
  input: PublishArtifact;
  version: number;
  title: string;
  destinationLabel: string;
  connectionName: string;
};
export function ArtifactPublications({
  artifactId,
  version,
  isCurrent,
  artifactType,
  projectId,
  canEdit,
}: {
  artifactId: string;
  version: ArtifactVersion;
  isCurrent: boolean;
  artifactType: ArtifactType;
  projectId?: string;
  canEdit: boolean;
}) {
  const cache = useQueryClient();
  const [offset, setOffset] = useState(0);
  const [connectionId, setConnectionId] = useState("");
  const [review, setReview] = useState<Review | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const previous = useRef<{ payload: string; key: string } | null>(null);
  const settings = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const destinations = useQuery({
    queryKey: ["artifact-destinations", projectId ?? "company"],
    queryFn: () => artifactsApi.destinations(projectId),
  });
  const publications = useQuery({
    queryKey: ["publications", artifactId, offset],
    queryFn: () => workToolsApi.publications(artifactId, offset),
    refetchInterval: (query) =>
      query.state.data?.items.some((job) =>
        ["queued", "processing"].includes(job.status),
      )
        ? 3000
        : false,
  });
  const publish = useMutation({
    mutationFn: (input: PublishArtifact) => {
      const payload = JSON.stringify([artifactId, input]);
      if (previous.current?.payload !== payload)
        previous.current = { payload, key: createIdempotencyKey() };
      return workToolsApi.publish(artifactId, input, previous.current.key);
    },
    onSuccess: (job) => {
      previous.current = null;
      setReview(null);
      setExpanded(job.public_id);
      setOffset(0);
      void cache.invalidateQueries({ queryKey: ["publications", artifactId] });
    },
  });
  const destination = destinations.data?.items.find(
    (item) => item.artifact_type === artifactType,
  );
  const connections =
    settings.data?.connections.filter(
      (connection) =>
        connection.enabled && connection.provider === destination?.provider,
    ) ?? [];
  const selected =
    connections.find((connection) => connection.public_id === connectionId) ??
    connections[0];
  const capability = settings.data?.capabilities.find(
    (item) => item.provider === destination?.provider,
  );
  const external =
    destination && destination.provider !== "internal" && destination.target_id;
  const canPublish =
    canEdit &&
    isCurrent &&
    version.status === "validated" &&
    Boolean(
      external &&
      selected &&
      settings.data?.storage_available &&
      capability?.create,
    );
  const already = publications.data?.items.find(
    (job) =>
      job.version_id === version.public_id &&
      job.provider === destination?.provider &&
      job.target_id === destination?.target_id,
  );
  function prepare() {
    if (!canPublish || !destination?.target_id || !selected) return;
    publish.reset();
    setReview({
      input: {
        version_id: version.public_id,
        connection_id: selected.public_id,
        expected_provider: destination.provider as WorkToolProvider,
        expected_target_id: destination.target_id,
      },
      version: version.version,
      title: version.title,
      destinationLabel: destination.label || destination.target_id,
      connectionName: selected.name,
    });
  }
  return (
    <section
      className="space-y-5 border-t pt-7"
      aria-labelledby="artifact-publications-title"
    >
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2
            id="artifact-publications-title"
            className="text-xl font-semibold"
          >
            Publications dans vos outils
          </h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Une version validée peut être publiée à votre demande.
            L’enregistrement d’une destination ne transmet aucun contenu.
          </p>
        </div>
        <Button asChild variant="outline">
          <Link
            to={
              projectId
                ? `/settings/destinations?project=${projectId}`
                : "/settings/destinations"
            }
          >
            Réglages de destination
          </Link>
        </Button>
      </div>
      {settings.isPending || destinations.isPending ? (
        <LoadingState label="Chargement des possibilités de publication…" />
      ) : settings.isError || destinations.isError ? (
        <ErrorState
          error={(settings.error ?? destinations.error)!}
          retry={() => {
            void settings.refetch();
            void destinations.refetch();
          }}
        />
      ) : (
        <div className="space-y-4 rounded-lg border p-5">
          {!isCurrent ? (
            <p className="text-sm text-muted-foreground">
              Cette version historique reste exportable. Ouvrez la version
              courante pour préparer une publication.
            </p>
          ) : version.status !== "validated" ? (
            <p className="text-sm text-muted-foreground">
              Validez la version courante avant de la publier dans un outil.
            </p>
          ) : !canEdit ? (
            <p className="text-sm text-muted-foreground">
              Un collaborateur ou propriétaire peut publier cette version.
            </p>
          ) : !external ? (
            <p className="text-sm text-muted-foreground">
              Le stockage interne est sélectionné. Choisissez une destination
              externe pour publier ; l’export reste disponible.
            </p>
          ) : !settings.data.storage_available ? (
            <p className="text-sm text-amber-900">
              Les connexions sécurisées ne sont pas disponibles sur cette
              instance.
            </p>
          ) : !selected ? (
            <p className="text-sm text-muted-foreground">
              Aucune connexion autorisée pour{" "}
              {providerLabels[destination!.provider]}.{" "}
              <Link to="/settings/tools" className="text-primary underline">
                Configurer les outils de l’équipe
              </Link>
              .
            </p>
          ) : !capability?.create ? (
            <p className="text-sm text-muted-foreground">
              La publication n’est pas disponible pour cet outil.
            </p>
          ) : (
            <>
              <p className="text-sm">
                Destination :{" "}
                <strong>
                  {providerLabels[destination!.provider]} ·{" "}
                  {destination!.label || destination!.target_id}
                </strong>
              </p>
              <p className="break-all text-xs text-muted-foreground">
                {destination!.target_id}
              </p>
              <div className="max-w-lg">
                <label
                  htmlFor="publication-connection"
                  className="mb-2 block text-sm font-medium"
                >
                  Connexion utilisée
                </label>
                <select
                  id="publication-connection"
                  className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
                  value={selected.public_id}
                  disabled={publish.isPending || Boolean(review)}
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
              {already ? (
                <div className="space-y-3">
                  <p className="text-sm">
                    Une demande existe déjà pour cette version et cette
                    destination :{" "}
                    {publicationLabels[already.status].toLowerCase()}.
                  </p>
                  <Button
                    variant="outline"
                    onClick={() => setExpanded(already.public_id)}
                  >
                    Consulter cette demande
                  </Button>
                </div>
              ) : !review ? (
                <Button onClick={prepare}>
                  Préparer la publication de la version {version.version}
                </Button>
              ) : null}
            </>
          )}
          {review && canEdit && isCurrent && version.status === "validated" ? (
            <div
              className="space-y-4 rounded-md border border-primary/25 bg-primary/5 p-5"
              role="region"
              aria-label="Confirmation de publication"
            >
              <h3 className="font-semibold">
                Confirmer l’envoi de ce livrable
              </h3>
              <dl className="grid gap-2 text-sm">
                <div>
                  <dt className="inline text-muted-foreground">Document : </dt>
                  <dd className="inline">
                    {review.title} · version {review.version}
                  </dd>
                </div>
                <div>
                  <dt className="inline text-muted-foreground">
                    Destination :{" "}
                  </dt>
                  <dd className="inline">
                    {providerLabels[review.input.expected_provider]} ·{" "}
                    {review.destinationLabel}
                    <span className="mt-1 block break-all font-mono text-xs">
                      {review.input.expected_target_id}
                    </span>
                  </dd>
                </div>
                <div>
                  <dt className="inline text-muted-foreground">Connexion : </dt>
                  <dd className="inline">{review.connectionName}</dd>
                </div>
              </dl>
              <p className="text-sm leading-6 text-muted-foreground">
                Le serveur créera une page ou un ticket dans cette destination.
                Le contenu, ses sources et la référence de cette publication
                seront transmis à l’outil.
              </p>
              <div className="flex flex-wrap gap-2">
                <Button
                  disabled={publish.isPending}
                  onClick={() => publish.mutate(review.input)}
                >
                  {publish.isPending
                    ? "Enregistrement de la demande…"
                    : "Confirmer la publication"}
                </Button>
                <Button
                  variant="ghost"
                  disabled={publish.isPending}
                  onClick={() => {
                    setReview(null);
                    publish.reset();
                  }}
                >
                  Annuler
                </Button>
              </div>
            </div>
          ) : null}
          {publish.error ? (
            <ErrorState
              title="La demande de publication n’a pas pu être confirmée"
              error={publish.error}
            />
          ) : null}
        </div>
      )}
      {publications.isPending ? (
        <LoadingState label="Chargement des publications…" />
      ) : publications.isError ? (
        <ErrorState
          error={publications.error}
          retry={() => void publications.refetch()}
        />
      ) : (
        <>
          <div className="space-y-4">
            {publications.data.items.map((job) => {
              const url = safeWorkToolUrl(job.provider, job.external_url);
              return (
                <article
                  key={job.public_id}
                  className="space-y-4 rounded-lg border p-5"
                >
                  <div className="flex flex-wrap items-start justify-between gap-4">
                    <div>
                      <h3 className="font-semibold">
                        {providerLabels[job.provider]} ·{" "}
                        {publicationLabels[job.status]}
                      </h3>
                      <p className="mt-1 break-all text-xs text-muted-foreground">
                        Destination : {job.target_id}
                      </p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {job.version_id === version.public_id
                          ? `Version affichée · ${version.version}`
                          : `Version ${job.version_id}`}{" "}
                        · {formatDate(job.created_at, true)}
                      </p>
                    </div>
                    {url ? (
                      <a
                        className="text-sm text-primary underline"
                        href={url}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        Ouvrir dans l’outil
                      </a>
                    ) : null}
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      setExpanded(
                        expanded === job.public_id ? null : job.public_id,
                      )
                    }
                    aria-expanded={expanded === job.public_id}
                  >
                    {expanded === job.public_id
                      ? "Masquer les observations"
                      : "Détails et observations"}
                  </Button>
                  {expanded === job.public_id ? (
                    <PublicationDetailPanel
                      publicationId={job.public_id}
                      canEdit={canEdit}
                    />
                  ) : null}
                </article>
              );
            })}
          </div>
          {!publications.data.items.length ? (
            <EmptyState
              title="Aucune publication enregistrée"
              description="Les exports sont disponibles sans publier ce livrable dans un outil externe."
            />
          ) : null}
          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - 10))}
            >
              Plus récentes
            </Button>
            <Button
              variant="outline"
              disabled={publications.data.items.length < 10}
              onClick={() => setOffset(offset + 10)}
            >
              Plus anciennes
            </Button>
          </div>
        </>
      )}
    </section>
  );
}
