import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { workToolsApi } from "@/api/work-tools";
import type { Publication } from "@/api/work-tool-types";
import { createIdempotencyKey } from "@/api/client";
import {
  publicationLabels,
  safeWorkToolUrl,
  validExternalObjectId,
} from "@/lib/work-tool-publication";
import { formatDate } from "@/lib/format";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function PublicationDetailPanel({
  publicationId,
  canEdit,
}: {
  publicationId: string;
  canEdit: boolean;
}) {
  const cache = useQueryClient();
  const [externalId, setExternalId] = useState("");
  const key = useRef<{ fingerprint: string; key: string } | null>(null);
  const detail = useQuery({
    queryKey: ["publication", publicationId],
    queryFn: () => workToolsApi.publication(publicationId),
    refetchInterval: (query) =>
      ["queued", "processing"].includes(
        query.state.data?.publication.status ?? "",
      )
        ? 3000
        : false,
  });
  const action = useMutation({
    mutationFn: (kind: "refresh" | "reconcile" | "cancel") => {
      const fingerprint = JSON.stringify([
        publicationId,
        kind,
        kind === "reconcile" ? externalId.trim() : null,
      ]);
      if (key.current?.fingerprint !== fingerprint)
        key.current = { fingerprint, key: createIdempotencyKey() };
      return kind === "reconcile"
        ? workToolsApi.reconcile(
            publicationId,
            externalId.trim(),
            key.current.key,
          )
        : kind === "cancel"
          ? workToolsApi.cancel(publicationId, key.current.key)
          : workToolsApi.refresh(publicationId, key.current.key);
    },
    onSuccess: (value: Publication) => {
      key.current = null;
      cache.setQueryData(["publication", publicationId], {
        ...detail.data,
        publication: value,
      });
      void cache.invalidateQueries({
        queryKey: ["publication", publicationId],
      });
      void cache.invalidateQueries({
        queryKey: ["publications", value.artifact_id],
      });
    },
  });
  if (detail.isPending)
    return <LoadingState label="Chargement des observations…" />;
  if (detail.isError)
    return (
      <ErrorState error={detail.error} retry={() => void detail.refetch()} />
    );
  const job = detail.data.publication;
  const url = safeWorkToolUrl(job.provider, job.external_url);
  return (
    <div className="space-y-4 border-t pt-5">
      <p className="text-sm font-medium">{publicationLabels[job.status]}</p>
      <p className="text-xs text-muted-foreground">
        Dernière mise à jour {formatDate(job.updated_at, true)} ·{" "}
        {job.attempt_count} tentative(s) distante(s).
      </p>
      {url ? (
        <a
          href={url}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-block break-all text-sm text-primary underline underline-offset-4"
        >
          Ouvrir l’objet dans l’outil
        </a>
      ) : null}
      {job.status === "needs_review" ? (
        <div className="space-y-4 rounded-lg border border-amber-200 bg-amber-50 p-4">
          <p className="text-sm leading-6 text-amber-950">
            La création a peut-être abouti, mais sa réponse n’a pas été
            confirmée. Vérifiez dans l’outil avant toute autre action. AI Center
            ne relance pas cette création.
          </p>
          {canEdit ? (
            <form
              className="space-y-3"
              onSubmit={(event) => {
                event.preventDefault();
                if (validExternalObjectId(job.provider, externalId.trim()))
                  action.mutate("reconcile");
              }}
            >
              <label
                htmlFor={`external-object-${publicationId}`}
                className="block text-sm font-medium"
              >
                {job.provider === "github"
                  ? "Numéro du ticket GitHub trouvé"
                  : "Identifiant de la page ou du ticket trouvé"}
              </label>
              <Input
                id={`external-object-${publicationId}`}
                value={externalId}
                disabled={action.isPending}
                onChange={(event) => setExternalId(event.target.value)}
                placeholder={
                  job.provider === "github"
                    ? "42"
                    : "Identifiant de l’objet dans l’outil"
                }
              />
              <p className="text-xs leading-5 text-amber-950">
                La vérification compare la destination et la référence AI Center
                de l’objet. Elle ne crée aucun nouveau document.
              </p>
              <Button
                type="submit"
                variant="outline"
                disabled={
                  action.isPending ||
                  !validExternalObjectId(job.provider, externalId.trim())
                }
              >
                Vérifier et rattacher cet objet
              </Button>
            </form>
          ) : null}
        </div>
      ) : null}
      {job.status === "conflict" ? (
        <p className="rounded-lg bg-amber-50 p-4 text-sm text-amber-950">
          Le contenu distant a changé depuis sa publication. Comparez les
          observations avant de décider de la suite ; AI Center ne l’écrase pas.
        </p>
      ) : null}
      {job.status === "unavailable" ? (
        <p className="rounded-lg bg-amber-50 p-4 text-sm text-amber-950">
          La dernière lecture n’a pas pu confirmer l’accès à l’objet. Les
          observations précédentes restent conservées.
        </p>
      ) : null}
      {job.status === "failed" ? (
        <p className="text-sm text-muted-foreground">
          La publication n’est pas confirmée. Vérifiez la connexion, la
          destination et le motif indiqué avant de préparer une autre action.
        </p>
      ) : null}
      {canEdit &&
      ["succeeded", "conflict", "unavailable"].includes(job.status) ? (
        <Button
          variant="outline"
          disabled={action.isPending}
          onClick={() => action.mutate("refresh")}
        >
          {action.isPending ? "Lecture en cours…" : "Relire l’objet distant"}
        </Button>
      ) : null}
      {canEdit && job.status === "queued" ? (
        <Button
          variant="outline"
          disabled={action.isPending}
          onClick={() => action.mutate("cancel")}
        >
          Annuler la demande en attente
        </Button>
      ) : null}
      {action.error ? (
        <ErrorState
          title="L’action n’a pas pu être confirmée"
          error={action.error}
        />
      ) : null}
      {job.error_code ? (
        <details className="text-xs text-muted-foreground">
          <summary>Motif signalé par le service</summary>
          <p className="mt-2 break-words">{job.error_code}</p>
        </details>
      ) : null}
      <div>
        <h4 className="text-sm font-semibold">Observations conservées</h4>
        {detail.data.observations.length ? (
          <ol className="mt-3 space-y-3">
            {detail.data.observations.map((observation) => (
              <li key={observation.public_id} className="rounded-md border p-4">
                <p className="text-sm">
                  {formatDate(observation.observed_at, true)}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  {observation.observation_kind} · objet{" "}
                  {observation.external_id}
                </p>
                <details className="mt-3 text-xs">
                  <summary>Contenu observé à cette date</summary>
                  <pre className="mt-3 max-h-80 overflow-auto whitespace-pre-wrap break-words leading-5">
                    {JSON.stringify(observation.snapshot, null, 2)}
                  </pre>
                </details>
              </li>
            ))}
          </ol>
        ) : (
          <p className="mt-2 text-sm text-muted-foreground">
            Aucune observation distante enregistrée pour le moment.
          </p>
        )}
      </div>
    </div>
  );
}
