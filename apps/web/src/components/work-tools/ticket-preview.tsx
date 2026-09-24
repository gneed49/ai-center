import { useEffect, useRef } from "react";
import type { TicketPreview } from "@/api/ticket-publication-types";
import { providerLabels } from "@/components/artifacts/labels";
import { Button } from "@/components/ui/button";
import { PublicationCard } from "./publication-card";
export function PreviewConfirmation({
  review,
  destinationLabel,
  connectionName,
  canConfirm,
  acknowledged,
  onAcknowledge,
  onConfirm,
  currentVersionId,
  canEdit,
}: {
  review: TicketPreview;
  destinationLabel: string;
  connectionName: string;
  canConfirm: boolean;
  acknowledged: boolean;
  onAcknowledge: (value: boolean) => void;
  onConfirm: () => void;
  currentVersionId: string;
  canEdit: boolean;
}) {
  const ref = useRef<HTMLElement | null>(null);
  useEffect(() => {
    ref.current?.focus();
  }, [review.preview_fingerprint]);
  const capacity = Math.min(
    review.capacity.available_pending,
    review.capacity.available_hourly,
  );
  return (
    <section
      ref={ref}
      tabIndex={-1}
      aria-label="Confirmation des tickets"
      className="space-y-4 rounded-lg border border-primary/30 bg-primary/5 p-5 outline-offset-4"
    >
      <h3 className="text-lg font-semibold">
        Vérifier les {review.requested_count} tickets sélectionnés
      </h3>
      <p className="text-sm">
        Version {review.source_version_number} ·{" "}
        {providerLabels[review.provider]} ·{" "}
        <span>{destinationLabel || "Destination configurée"}</span>
      </p>
      <p className="text-sm">Connexion : {connectionName}</p>
      <details className="text-xs text-muted-foreground">
        <summary>Identifiant de la destination</summary>
        <p className="break-all">{review.target_id}</p>
      </details>
      <p className="text-sm">
        {review.new_count} nouveau(x) ticket(s) seront créés ;{" "}
        {review.existing_count} demande(s) existante(s) seront conservées.
      </p>
      <p className="text-sm">
        Capacité actuelle : {review.capacity.available_pending} place(s) en
        attente, {review.capacity.available_hourly} création(s) dans la limite
        horaire. Cette disponibilité peut changer avant confirmation.
      </p>
      {review.new_count > capacity ? (
        <p role="alert" className="text-sm text-amber-950">
          La capacité est insuffisante pour cette sélection. Choisissez moins de
          tickets ou attendez, puis préparez un nouvel aperçu.
        </p>
      ) : null}
      <ol className="space-y-3">
        {review.items.map((item) => (
          <li
            key={item.source_ticket_index}
            className="min-w-0 rounded-md border bg-background p-4"
          >
            <h4 className="break-words font-medium">
              Ticket {item.source_ticket_index + 1} — {item.title}
            </h4>
            <details className="mt-3 text-sm">
              <summary>Contenu exact transmis à l’outil</summary>
              <pre className="mt-3 whitespace-pre-wrap break-words font-sans leading-6">
                {item.business_body_markdown}
              </pre>
            </details>
            {item.existing_publication ? (
              <p className="mt-2 text-sm">
                Une demande existe déjà pour cette entrée ; elle sera conservée.
              </p>
            ) : null}
          </li>
        ))}
      </ol>
      <p className="text-xs text-muted-foreground">
        Une référence technique AI Center sera ajoutée à chaque ticket pour
        vérifier son reçu. Les autres tickets du document ne sont pas transmis
        dans cette issue.
      </p>
      {review.prior_publications.length ? (
        <div className="space-y-3">
          <h4 className="font-medium">
            Publications déjà présentes pour ce livrable
          </h4>
          <p className="text-sm">
            Les nouvelles issues s’ajouteront aux anciennes. Elles ne les
            remplaceront pas et ne les mettront pas à jour.
          </p>
          {review.prior_publications.map((job) => (
            <PublicationCard
              key={job.public_id}
              publication={job}
              currentVersionId={currentVersionId}
              canEdit={canEdit}
            />
          ))}
        </div>
      ) : null}
      {review.requires_additional_confirmation ? (
        <label className="flex items-start gap-3 text-sm">
          <input
            type="checkbox"
            className="mt-1 size-4 shrink-0"
            checked={acknowledged}
            onChange={(event) => onAcknowledge(event.target.checked)}
          />
          Je confirme la création de nouveaux tickets ; les tickets existants ne
          seront pas mis à jour.
        </label>
      ) : null}
      {review.new_count > 0 ? (
        <Button
          disabled={
            !canConfirm ||
            review.new_count > capacity ||
            (review.requires_additional_confirmation && !acknowledged)
          }
          onClick={onConfirm}
        >
          Confirmer la création de {review.new_count} tickets dans{" "}
          {providerLabels[review.provider]}
        </Button>
      ) : (
        <p className="text-sm">
          Toutes les entrées disposent déjà d’une demande. Consultez leur état
          dans la liste des tickets.
        </p>
      )}
    </section>
  );
}
