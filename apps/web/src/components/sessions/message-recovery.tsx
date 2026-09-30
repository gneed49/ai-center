import type { MessageCommandView } from "@/api/types";
import { Button } from "@/components/ui/button";

const labels: Record<MessageCommandView["status"], string> = {
  processing: "Réponse en préparation",
  interrupted: "Demande interrompue",
  retryable: "Réponse à reprendre",
  failed: "Demande non aboutie",
  completed: "Réponse disponible",
  expired: "Reprise expirée",
};
export function MessageRecovery({
  commands,
  busy,
  readOnly,
  onRetry,
  onRefresh,
}: {
  commands: MessageCommandView[];
  busy: boolean;
  readOnly: boolean;
  onRetry: (command: MessageCommandView) => void;
  onRefresh: () => void;
}) {
  const pending = commands.filter((command) => command.status !== "completed");
  if (!pending.length) return null;
  return (
    <section className="space-y-3" aria-label="Suivi de vos demandes">
      <h2 className="text-sm font-semibold">Vos demandes à suivre</h2>
      {pending.map((command) => (
        <article
          key={command.idempotency_key}
          className="space-y-3 rounded-md border border-amber-200 bg-amber-50 p-4 text-sm"
        >
          <p className="font-medium">{labels[command.status]}</p>
          <p className="whitespace-pre-wrap break-words text-amber-950">
            {command.submitted_content}
          </p>
          {command.error_message ? (
            <p className="text-xs text-amber-900">{command.error_message}</p>
          ) : null}
          {command.status === "processing" ? (
            <p className="text-xs text-amber-900">
              Le serveur traite cette demande. Son état est actualisé sans
              relancer de génération.
            </p>
          ) : null}
          {command.retry_after_seconds !== null &&
          command.retry_after_seconds > 0 ? (
            <p className="text-xs text-amber-900">
              Prochain essai possible dans environ {command.retry_after_seconds}{" "}
              seconde(s).
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            {command.can_retry && !readOnly ? (
              <Button
                size="sm"
                disabled={busy}
                onClick={() => onRetry(command)}
              >
                Reprendre cette demande
              </Button>
            ) : null}
            <Button size="sm" variant="outline" onClick={onRefresh}>
              Actualiser son état
            </Button>
          </div>
          {!command.can_retry && command.status !== "processing" ? (
            <p className="text-xs text-amber-900">
              Cette demande ne peut pas être relancée depuis votre accès actuel.
              Son historique reste conservé.
            </p>
          ) : null}
        </article>
      ))}
    </section>
  );
}
