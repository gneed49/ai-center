import { useMutation } from "@tanstack/react-query";
import { useRef } from "react";
import { createIdempotencyKey } from "@/api/client";
import { workToolsApi } from "@/api/work-tools";
import type { WorkToolConnection } from "@/api/work-tool-types";
import { providerLabels } from "@/components/artifacts/labels";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/app/page";

export function WorkToolConnectionCard({
  connection,
  owner,
  storage,
  onEdit,
  onChanged,
}: {
  connection: WorkToolConnection;
  owner: boolean;
  storage: boolean;
  onEdit: () => void;
  onChanged: (connection: WorkToolConnection) => void;
}) {
  const testKey = useRef<string | null>(null);
  const disableKey = useRef<string | null>(null);
  const test = useMutation({
    mutationFn: () => {
      testKey.current ??= createIdempotencyKey();
      return workToolsApi.test(connection.public_id, testKey.current);
    },
    onSuccess: () => {
      testKey.current = null;
    },
  });
  const disable = useMutation({
    mutationFn: () => {
      disableKey.current ??= createIdempotencyKey();
      return workToolsApi.disable(
        connection.public_id,
        connection.revision,
        disableKey.current,
      );
    },
    onSuccess: (value) => {
      disableKey.current = null;
      onChanged(value);
    },
  });
  const busy = test.isPending || disable.isPending;
  return (
    <article className="space-y-4 rounded-lg border p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-xs font-medium text-primary">
            {providerLabels[connection.provider]}
          </p>
          <h2 className="mt-1 break-words text-lg font-semibold">
            {connection.name}
          </h2>
        </div>
        <span className="rounded-full bg-muted px-3 py-1 text-xs">
          {connection.enabled ? "Autorisée pour l’équipe" : "Désactivée"}
        </span>
      </div>
      <p className="text-sm text-muted-foreground">
        {connection.enabled
          ? "Une clé est enregistrée. Le test d’accès contacte l’outil à votre demande ; il ne vérifie pas les droits sur chaque destination."
          : "Cette connexion ne peut plus servir à une nouvelle publication."}
      </p>
      {owner ? (
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            disabled={!storage || !connection.enabled || busy}
            onClick={() => test.mutate()}
          >
            {test.isPending ? "Vérification…" : "Tester l’accès"}
          </Button>
          <Button
            variant="outline"
            disabled={!storage || busy}
            onClick={onEdit}
          >
            Modifier
          </Button>
          {connection.enabled ? (
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => disable.mutate()}
            >
              Désactiver
            </Button>
          ) : null}
        </div>
      ) : null}
      {test.data ? (
        <p
          role="status"
          className={`text-sm ${test.data.ok ? "text-emerald-800" : "text-amber-900"}`}
        >
          {test.data.ok
            ? "L’outil reconnaît cette clé. Les permissions de la destination seront vérifiées lors d’une action sur celle-ci."
            : "L’accès n’a pas pu être vérifié. Vérifiez la clé et ses permissions dans l’outil."}
        </p>
      ) : null}
      {test.error || disable.error ? (
        <ErrorState error={(test.error ?? disable.error)!} />
      ) : null}
    </article>
  );
}
