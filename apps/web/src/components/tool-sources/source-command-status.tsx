import { Link } from "react-router";
import { observationPath } from "@/api/tool-sources";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/app/page";
import { sourceDate, sourceTitle } from "./source-labels";
import type { SourceCommandController } from "./use-source-command";
const statusLabels = {
  not_received: "Le serveur n’a pas enregistré cette demande.",
  processing: "Lecture en cours. Vous pouvez revenir consulter son résultat.",
  interrupted: "La lecture a été interrompue.",
  retryable: "Cette demande peut être reprise.",
  failed: "La demande n’a pas abouti.",
  expired: "Cette demande a expiré. Sa clé ne peut plus être réutilisée.",
  completed: "Le résultat de cette demande est enregistré.",
};
const effectLabels = {
  created: "Source ajoutée",
  existing: "Cette source est déjà rattachée",
  changed: "Une nouvelle lecture a été enregistrée",
  unchanged: "Contenu inchangé",
  detached: "Source retirée du contexte actif",
  rebound: "Connexion vérifiée et source réactivée",
};
export function SourceCommandStatus({
  command,
  canAct,
}: {
  command: SourceCommandController;
  canAct: boolean;
}) {
  const receipt = command.receipt.error ? undefined : command.receipt.data;
  const result = receipt?.result;
  if (!command.saved)
    return command.error ? <ErrorState error={command.error} /> : null;
  const path = result
    ? observationPath(
        result.observation.source_kind,
        result.observation.public_id,
      )
    : null;
  return (
    <section
      className="space-y-4 rounded-lg border border-indigo-200 bg-indigo-50/40 p-5"
      aria-label="Suivi de la demande de source"
    >
      <h2 className="font-semibold">Suivi de votre demande</h2>
      <p role="status" className="text-sm">
        {command.busy
          ? "Lecture en cours…"
          : receipt
            ? statusLabels[receipt.status]
            : "Vérification du résultat enregistré…"}
      </p>
      {result ? (
        <div className="space-y-2 text-sm">
          <p className="font-medium">
            {effectLabels[result.effect]} ·{" "}
            {sourceTitle(result.observation.title)}
          </p>
          <p>
            Contenu lu le {sourceDate(result.observation.observed_at)}
            {result.verification_status === "not_performed"
              ? " · Aucune nouvelle lecture n’a été effectuée."
              : ""}
          </p>
          {result.verification_status === "partial" ? (
            <p>
              Lecture partielle : consultez ses limites avant de l’utiliser.
            </p>
          ) : null}
          {path ? (
            <Link className="block text-primary underline" to={path}>
              Ouvrir la lecture de cette demande
            </Link>
          ) : null}
          <Link
            className="block text-primary underline"
            to={`/sources/${result.reference.public_id}`}
          >
            Voir l’état actuel de la source
          </Link>
        </div>
      ) : null}
      {receipt?.error ? (
        <p role="alert" className="text-sm">
          {receipt.error.message}
        </p>
      ) : null}
      {receipt?.retry_after ? (
        <p className="text-sm">
          Nouvelle tentative possible après {sourceDate(receipt.retry_after)}.
          Le serveur vérifiera la disponibilité au moment de votre demande.
        </p>
      ) : null}
      {command.error && receipt?.status !== "completed" ? (
        <ErrorState
          error={command.error}
          title="La demande n’a pas pu être confirmée"
        />
      ) : null}
      {command.receipt.error ? (
        <ErrorState
          error={command.receipt.error}
          title="Impossible de vérifier le résultat pour le moment"
        />
      ) : null}
      <div className="flex flex-wrap gap-3">
        <Button
          variant="outline"
          disabled={command.receipt.isFetching}
          onClick={() => void command.receipt.refetch()}
        >
          Vérifier le résultat enregistré
        </Button>
        {canAct && receipt?.can_retry && receipt.status !== "processing" ? (
          <Button disabled={command.busy} onClick={command.retry}>
            Reprendre cette demande
          </Button>
        ) : null}
        {receipt &&
        ["completed", "failed", "expired"].includes(receipt.status) ? (
          <Button
            variant="ghost"
            disabled={command.busy}
            onClick={command.clear}
          >
            Préparer une nouvelle demande
          </Button>
        ) : null}
      </div>
      {receipt?.can_retry ? (
        <p className="text-xs text-muted-foreground">
          Une reprise explicite peut refaire la lecture et consommer le budget
          de l’équipe. Consulter ce reçu ne contacte pas l’outil.
        </p>
      ) : null}
    </section>
  );
}
