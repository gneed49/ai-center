import { useEffect, useRef, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { workToolsApi } from "@/api/work-tools";
import type {
  PreviewTickets,
  TicketPublicationResult,
} from "@/api/ticket-publication-types";
import type { ArtifactType, ArtifactVersion } from "@/api/artifact-types";
import { typedDraft } from "@/components/artifacts/typed-draft";
import { providerLabels } from "@/components/artifacts/labels";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { PublicationCard } from "./publication-card";
import { PreviewConfirmation } from "./ticket-preview";
import { TicketHistory } from "./ticket-history";
import {
  newTicketCommand,
  readTicketCommand,
  saveTicketCommand,
  ticketCommandKey,
  type TicketCommand,
} from "./ticket-command";

type Props = {
  artifactId: string;
  version: ArtifactVersion;
  isCurrent: boolean;
  artifactType: ArtifactType;
  projectId?: string;
  canEdit: boolean;
  currentVersionId?: string;
  currentVersionNumber?: number;
  children: ReactNode;
};
export function TicketPublications(props: Props) {
  const storageKey = ticketCommandKey(props.artifactId, props.projectId);
  return <Workflow key={storageKey} {...props} storageKey={storageKey} />;
}
function Workflow({
  artifactId,
  version,
  isCurrent,
  artifactType,
  projectId,
  canEdit,
  currentVersionId,
  currentVersionNumber,
  children,
  storageKey,
}: Props & { storageKey: string }) {
  const cache = useQueryClient();
  const [searchParams] = useSearchParams();
  const requestedTicket = searchParams.has("ticket");
  const settings = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const destinations = useQuery({
    queryKey: ["artifact-destinations", projectId ?? "company"],
    queryFn: () => artifactsApi.destinations(projectId),
  });
  const destination = destinations.data?.items.find(
    (d) => d.artifact_type === artifactType,
  );
  const issueMode =
    destination?.provider === "linear" || destination?.provider === "github";
  const connections =
    settings.data?.connections.filter(
      (c) => c.enabled && c.provider === destination?.provider,
    ) ?? [];
  const [connectionId, setConnectionId] = useState("");
  const connection =
    connections.find((c) => c.public_id === connectionId) ?? connections[0];
  const draft = typedDraft(version.structured_content);
  const coverage = useQuery({
    queryKey: [
      "ticket-coverage",
      artifactId,
      version.public_id,
      destination?.provider,
      destination?.target_id,
    ],
    enabled: Boolean(
      issueMode && destination?.target_id && draft?.tickets.length,
    ),
    queryFn: () =>
      workToolsApi.ticketCoverage(
        artifactId,
        version.public_id,
        destination!.provider,
        destination!.target_id!,
      ),
    refetchInterval: (query) =>
      query.state.data?.items.some((item) =>
        ["queued", "processing"].includes(
          item.existing_publication?.status ?? "",
        ),
      )
        ? 3000
        : false,
  });
  const completeCoverage = Boolean(
    draft &&
    coverage.data?.version_id === version.public_id &&
    coverage.data.items.length === draft.tickets.length &&
    draft.tickets.every((_, i) =>
      coverage.data!.items.some((item) => item.source_ticket_index === i),
    ),
  );
  const [selected, setSelected] = useState<number[]>([]);
  const eligible =
    coverage.data?.items
      .filter((item) => !item.existing_publication)
      .map((item) => item.source_ticket_index) ?? [];
  const indexes = selected
    .filter((index) => eligible.includes(index))
    .sort((a, b) => a - b);
  const [acknowledged, setAcknowledged] = useState(false);
  const [saved, setSaved] = useState(() => readTicketCommand(storageKey));
  const commandRef = useRef(saved);
  const input: PreviewTickets | null =
    issueMode && connection && destination?.target_id
      ? {
          version_id: version.public_id,
          connection_id: connection.public_id,
          expected_provider: connection.provider,
          expected_target_id: destination.target_id,
          ticket_indexes: indexes,
        }
      : null;
  const preview = useMutation({
    mutationFn: (value: PreviewTickets) =>
      workToolsApi.ticketPreview(artifactId, value),
    retry: false,
  });
  const samePreparation =
    input && JSON.stringify(preview.variables) === JSON.stringify(input);
  const review =
    samePreparation &&
    preview.data?.connection_revision === connection?.revision
      ? preview.data
      : null;
  const receipt = useQuery({
    queryKey: ["ticket-publication-receipt", artifactId, saved?.key],
    enabled: Boolean(saved),
    queryFn: () => workToolsApi.ticketReceipt(artifactId, saved!.key),
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.status === "processing" ? 2000 : false,
  });
  const resultRef = useRef<HTMLElement | null>(null);
  const publish = useMutation({
    mutationFn: (command: TicketCommand) =>
      workToolsApi.publishTickets(artifactId, command.input, command.key),
    retry: false,
    onSuccess: (result: TicketPublicationResult, command) => {
      cache.setQueryData(
        ["ticket-publication-receipt", artifactId, command.key],
        { status: "completed", can_retry: false, result },
      );
      void cache.invalidateQueries({
        queryKey: ["ticket-coverage", artifactId],
      });
      void cache.invalidateQueries({ queryKey: ["publications", artifactId] });
      void cache.invalidateQueries({ queryKey: ["work-tools"] });
    },
    onSettled: (_data, _error, command) =>
      void cache.invalidateQueries({
        queryKey: ["ticket-publication-receipt", artifactId, command.key],
      }),
  });
  useEffect(() => {
    // A restored receipt must not steal focus from an explicit source link.
    if (receipt.data?.result && !requestedTicket) resultRef.current?.focus();
  }, [receipt.data?.result, requestedTicket]);
  const capable =
    settings.data?.storage_available &&
    settings.data.capabilities.some(
      (cap) => cap.provider === destination?.provider && cap.create,
    );
  const canPrepare = Boolean(
    canEdit &&
    isCurrent &&
    version.status === "validated" &&
    capable &&
    connection &&
    completeCoverage &&
    !saved,
  );
  function resetSelection(next: number[]) {
    setSelected(next);
    setAcknowledged(false);
    preview.reset();
  }
  function confirm() {
    if (
      !input ||
      !review ||
      !canPrepare ||
      (review.requires_additional_confirmation && !acknowledged)
    )
      return;
    const command =
      commandRef.current ??
      newTicketCommand(
        {
          ...input,
          preview_fingerprint: review.preview_fingerprint,
          prior_publications_fingerprint: review.prior_publications_fingerprint,
          confirm_additional_issues: acknowledged,
        },
        review.source_version_number,
      );
    try {
      saveTicketCommand(storageKey, command);
    } catch {
      setStorageError(true);
      return;
    }
    commandRef.current = command;
    setSaved(command);
    publish.mutate(command);
  }
  const [storageError, setStorageError] = useState(false);
  const result = receipt.data?.result;
  const latestId = currentVersionId ?? version.public_id;
  const pendingKnown =
    receipt.data?.status === "processing" || publish.isPending;
  const canStartAgain =
    saved &&
    receipt.data &&
    !pendingKnown &&
    ["completed", "failed", "expired"].includes(receipt.data.status);
  return (
    <>
      {saved ? (
        <section
          ref={resultRef}
          tabIndex={-1}
          className="space-y-4 rounded-lg border p-5 outline-offset-4"
          aria-label="Suivi des tickets demandés"
        >
          <h2 className="text-lg font-semibold">
            Suivi de votre demande · version {saved.sourceVersionNumber}
          </h2>
          <p className="text-sm">
            {saved.input.ticket_indexes.length} ticket(s) sélectionné(s) vers{" "}
            {providerLabels[saved.input.expected_provider]}.
          </p>
          {saved.input.version_id !== version.public_id ? (
            <p className="rounded-md bg-amber-50 p-3 text-sm text-amber-950">
              Cette demande concerne une autre version. Elle conserve sa
              sélection et sa destination initiales.{" "}
              <Link
                className="underline"
                to={`/artifacts/${artifactId}?version=${saved.input.version_id}`}
              >
                Ouvrir sa version source
              </Link>
              .
            </p>
          ) : null}
          {result ? (
            <>
              <p className="text-sm" role="status">
                {result.created_count} nouvelle(s) demande(s) enregistrée(s),{" "}
                {result.existing_count} demande(s) déjà existante(s). Les états
                ci-dessous indiquent les résultats dans l’outil.
              </p>
              <div className="grid gap-3">
                {result.publications.map((job) => (
                  <PublicationCard
                    key={job.public_id}
                    publication={job}
                    currentVersionId={latestId}
                    canEdit={canEdit}
                    live
                  />
                ))}
              </div>
            </>
          ) : (
            <p className="text-sm" role="status">
              {pendingKnown
                ? "Demande en cours. Vous pouvez revenir ici ; aucune création supplémentaire ne sera lancée au rechargement."
                : receipt.data?.status === "expired"
                  ? "Le délai de reprise est dépassé. Consultez les publications avant de préparer une autre sélection."
                  : "Votre demande est conservée. Vérifiez son état avant de la reprendre."}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => void receipt.refetch()}>
              Actualiser le reçu
            </Button>
            {canEdit && receipt.data?.can_retry && !pendingKnown ? (
              <Button onClick={() => publish.mutate(saved)}>
                Reprendre la même demande
              </Button>
            ) : null}
            {canStartAgain ? (
              <Button
                variant="ghost"
                onClick={() => {
                  localStorage.removeItem(storageKey);
                  commandRef.current = null;
                  setSaved(null);
                  resetSelection([]);
                  publish.reset();
                }}
              >
                Préparer une autre sélection
              </Button>
            ) : null}
          </div>
          {receipt.error ? (
            <ErrorState
              title="Le reçu est indisponible ; votre demande reste conservée"
              error={receipt.error}
            />
          ) : null}
          {publish.error ? (
            <ErrorState
              title="La demande n’a pas pu être confirmée"
              error={publish.error}
            />
          ) : null}
        </section>
      ) : null}
      {settings.isPending || destinations.isPending ? (
        <LoadingState label="Chargement des destinations de tickets…" />
      ) : settings.error || destinations.error ? (
        <ErrorState error={(settings.error ?? destinations.error)!} />
      ) : !issueMode ? (
        children
      ) : (
        <section
          aria-label="Publication des tickets"
          className="space-y-5 border-t pt-7"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h2 className="text-xl font-semibold">Tickets dans vos outils</h2>
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
          <p className="text-sm">
            Chaque ticket sélectionné crée sa propre issue dans{" "}
            {providerLabels[destination.provider]}. Destination :{" "}
            <strong>{destination.label || destination.target_id}</strong>.
          </p>
          {!isCurrent ? (
            <p className="text-sm">
              Cette version historique reste consultable. Préparez une
              publication depuis la version courante validée.
            </p>
          ) : version.status !== "validated" ? (
            <p className="text-sm">
              Validez la version courante avant de publier ses tickets.
            </p>
          ) : !canEdit ? (
            <p className="text-sm">
              Un collaborateur ou propriétaire peut publier ces tickets.
            </p>
          ) : null}
          {currentVersionNumber && currentVersionNumber > version.version ? (
            <p className="text-sm text-amber-900">
              Une version plus récente existe : version {currentVersionNumber}.
            </p>
          ) : null}
          {!draft?.tickets.length ? (
            <p className="rounded-md border p-4 text-sm">
              Ce document ne contient pas de tickets structurés utilisables.
              Préparez un brouillon de tickets avec un agent, puis relisez et
              validez sa version.
            </p>
          ) : !destination.target_id ? (
            <p>Configurez une destination de tickets avant de continuer.</p>
          ) : (
            <>
              {!connection || !capable ? (
                <p className="text-sm">
                  Aucune connexion disponible pour publier dans cet outil.{" "}
                  <Link className="text-primary underline" to="/settings/tools">
                    Configurer les connexions
                  </Link>
                  .
                </p>
              ) : (
                <label className="block max-w-lg space-y-2 text-sm font-medium">
                  Connexion utilisée
                  <select
                    className="min-h-11 w-full rounded-md border bg-white px-3"
                    value={connection.public_id}
                    disabled={
                      Boolean(saved) || publish.isPending || preview.isPending
                    }
                    onChange={(event) => {
                      setConnectionId(event.target.value);
                      resetSelection([]);
                    }}
                  >
                    {connections.map((item) => (
                      <option key={item.public_id} value={item.public_id}>
                        {item.name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              {coverage.isPending ? (
                <LoadingState label="Chargement de tous les tickets et de leurs publications…" />
              ) : coverage.error ? (
                <ErrorState
                  error={coverage.error}
                  retry={() => void coverage.refetch()}
                />
              ) : !completeCoverage ? (
                <p role="alert">
                  La liste des publications est incomplète. Actualisez-la avant
                  de préparer un envoi.{" "}
                  <Button
                    variant="outline"
                    onClick={() => void coverage.refetch()}
                  >
                    Actualiser tous les tickets
                  </Button>
                </p>
              ) : (
                <>
                  <div className="flex flex-wrap items-center gap-3">
                    <p className="text-sm">
                      {indexes.length} ticket(s) sélectionné(s) sur{" "}
                      {draft.tickets.length}.
                    </p>
                    <Button
                      variant="outline"
                      disabled={!canPrepare || preview.isPending}
                      onClick={() => resetSelection(eligible)}
                    >
                      Tout sélectionner
                    </Button>
                    <Button
                      variant="ghost"
                      disabled={!canPrepare || preview.isPending}
                      onClick={() => resetSelection([])}
                    >
                      Tout désélectionner
                    </Button>
                  </div>
                  <div className="grid gap-4">
                    {coverage.data!.items.map((item) => (
                      <div
                        key={item.source_ticket_index}
                        className="min-w-0 space-y-3 rounded-lg border p-4"
                      >
                        {item.existing_publication ? (
                          <PublicationCard
                            publication={item.existing_publication}
                            currentVersionId={latestId}
                            canEdit={canEdit}
                          />
                        ) : (
                          <>
                            <label className="flex items-start gap-3 text-sm font-medium">
                              <input
                                type="checkbox"
                                className="mt-1 size-4 shrink-0"
                                checked={indexes.includes(
                                  item.source_ticket_index,
                                )}
                                disabled={!canPrepare || preview.isPending}
                                onChange={(event) =>
                                  resetSelection(
                                    event.target.checked
                                      ? [...indexes, item.source_ticket_index]
                                      : indexes.filter(
                                          (index) =>
                                            index !== item.source_ticket_index,
                                        ),
                                  )
                                }
                              />
                              Sélectionner Ticket {item.source_ticket_index + 1}{" "}
                              — {item.title}
                            </label>
                            <ul className="list-disc space-y-1 pl-7 text-sm">
                              {draft.tickets[
                                item.source_ticket_index
                              ]?.acceptance_criteria.map((criterion, i) => (
                                <li key={i}>{criterion}</li>
                              ))}
                            </ul>
                          </>
                        )}
                      </div>
                    ))}
                  </div>
                  <Button
                    disabled={
                      !canPrepare || !indexes.length || preview.isPending
                    }
                    onClick={() => {
                      setAcknowledged(false);
                      if (input) preview.mutate(input);
                    }}
                  >
                    {preview.isPending
                      ? "Préparation de l’aperçu…"
                      : `Préparer les ${indexes.length} tickets`}
                  </Button>
                  {preview.error ? (
                    <ErrorState
                      title="L’aperçu doit être préparé à nouveau"
                      error={preview.error}
                    />
                  ) : null}
                  {review && !saved ? (
                    <PreviewConfirmation
                      review={review}
                      destinationLabel={destination.label}
                      connectionName={
                        connection?.name ?? "Connexion configurée"
                      }
                      canConfirm={canPrepare && !publish.isPending}
                      acknowledged={acknowledged}
                      onAcknowledge={setAcknowledged}
                      onConfirm={confirm}
                      currentVersionId={latestId}
                      canEdit={canEdit}
                    />
                  ) : null}
                  {storageError ? (
                    <p role="alert" className="text-sm">
                      La demande ne peut pas être conservée dans ce navigateur.
                      Autorisez le stockage local avant de confirmer.
                    </p>
                  ) : null}
                </>
              )}
            </>
          )}
          <TicketHistory
            artifactId={artifactId}
            currentVersionId={latestId}
            canEdit={canEdit}
          />
        </section>
      )}
    </>
  );
}
