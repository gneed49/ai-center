import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef } from "react";
import { Pause, Play } from "lucide-react";
import { companyControlsApi } from "@/api/company-controls";
import type {
  AutomationCommand,
  AutomationSettings,
} from "@/api/company-controls";
import { createIdempotencyKey } from "@/api/client";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
export function AutomationControls({ owner }: { owner: boolean }) {
  const cache = useQueryClient();
  const settings = useQuery({
    queryKey: ["automation"],
    queryFn: companyControlsApi.automation,
  });
  const previous = useRef<{ fingerprint: string; key: string } | null>(null);
  const change = useMutation({
    mutationFn: (input: AutomationCommand) => {
      const fingerprint = JSON.stringify(input);
      if (previous.current?.fingerprint !== fingerprint)
        previous.current = { fingerprint, key: createIdempotencyKey() };
      return companyControlsApi.setAutomation(input, previous.current.key);
    },
    onSuccess: async (receipt) => {
      previous.current = null;
      cache.setQueryData<AutomationSettings>(["automation"], (current) =>
        current ? { ...current, ...receipt } : current,
      );
      await cache.invalidateQueries({ queryKey: ["automation"] });
      await cache.invalidateQueries({ queryKey: ["work-tools"] });
    },
  });
  return (
    <section
      className="space-y-5 border-t pt-8"
      aria-labelledby="automation-controls-title"
    >
      <div>
        <h2 id="automation-controls-title" className="text-xl font-semibold">
          Contrôler l’automatisation
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">
          Suivez les appels IA et suspendez les traitements automatiques de
          l’entreprise, y compris les publications.
        </p>
      </div>
      {settings.isPending ? (
        <LoadingState label="Chargement de l’activité…" />
      ) : settings.isError ? (
        <ErrorState
          error={settings.error}
          retry={() => void settings.refetch()}
        />
      ) : (
        <div className="space-y-5 rounded-lg border p-5">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <p className="font-semibold">
              {settings.data.enabled
                ? "Automatisation autorisée"
                : "Automatisation en pause"}
            </p>
            {owner ? (
              <Button
                variant={settings.data.enabled ? "outline" : "default"}
                disabled={change.isPending}
                onClick={() =>
                  change.mutate({
                    enabled: !settings.data.enabled,
                    expected_generation: settings.data.generation,
                  })
                }
              >
                {settings.data.enabled ? <Pause /> : <Play />}
                {change.isPending
                  ? "Changement en cours…"
                  : settings.data.enabled
                    ? "Mettre l’automatisation en pause"
                    : "Reprendre l’automatisation"}
              </Button>
            ) : (
              <p className="text-xs text-muted-foreground">
                Le propriétaire contrôle la pause et la reprise.
              </p>
            )}
          </div>
          <dl className="grid gap-5 text-sm sm:grid-cols-3">
            <div>
              <dt className="text-muted-foreground">
                Appels sur la dernière heure
              </dt>
              <dd className="mt-1 font-semibold">
                {settings.data.calls_last_hour} /{" "}
                {settings.data.limits.calls_per_hour}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Appels simultanés</dt>
              <dd className="mt-1 font-semibold">
                {settings.data.active_calls} /{" "}
                {settings.data.limits.concurrent_calls}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Coût estimé connu</dt>
              <dd className="mt-1 font-semibold">
                {settings.data.known_estimated_cost_usd === null
                  ? "Non disponible"
                  : new Intl.NumberFormat("fr-FR", {
                      style: "currency",
                      currency: "USD",
                    }).format(settings.data.known_estimated_cost_usd)}
              </dd>
              <dd className="mt-1 text-xs text-muted-foreground">
                {settings.data.runs_without_cost} appel(s) sans estimation
              </dd>
            </div>
          </dl>
          {settings.data.limits.per_actor_calls_per_hour !== undefined && (
            <p className="text-sm text-muted-foreground">
              Votre utilisation : {settings.data.actor_calls_last_hour ?? "—"} /{" "}
              {settings.data.limits.per_actor_calls_per_hour} appels sur la
              dernière heure ; {settings.data.actor_active_calls ?? "—"} /{" "}
              {settings.data.limits.per_actor_concurrent_calls} simultanés. Les
              limites de l’entreprise s’appliquent aussi.
            </p>
          )}
          <p className="text-xs leading-5 text-muted-foreground">
            Durée maximale par appel :{" "}
            {settings.data.limits.call_timeout_seconds} secondes. Une estimation
            absente ne signifie pas un coût nul.
          </p>
          <p className="rounded-md bg-amber-50 p-4 text-sm leading-6 text-amber-950">
            La pause demande l’arrêt des appels locaux et bloque les nouveaux
            traitements. Les demandes déjà acceptées par un service externe
            peuvent encore produire un coût ou un résultat ; elles ne peuvent
            pas être retirées rétroactivement.
          </p>
          {change.error ? (
            <ErrorState
              title="Le changement n’a pas pu être confirmé"
              error={change.error}
              retry={
                change.variables
                  ? () => change.mutate(change.variables!)
                  : undefined
              }
            />
          ) : null}
        </div>
      )}
    </section>
  );
}
