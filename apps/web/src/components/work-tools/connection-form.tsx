import { useRef, useState } from "react";
import type { FormEvent } from "react";
import { createIdempotencyKey } from "@/api/client";
import { workToolsApi } from "@/api/work-tools";
import type {
  WorkToolConnection,
  WorkToolProvider,
} from "@/api/work-tool-types";
import { providerLabels } from "@/components/artifacts/labels";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { captureRequestContext } from "@/api/request-context";
import { retryIdentity, type RetryIdentity } from "@/lib/provider-settings";
import { useProviderAction } from "@/components/providers/use-provider-action";

export function WorkToolConnectionForm({
  connection,
  onSaved,
  onCancel,
}: {
  connection?: WorkToolConnection;
  onSaved: (connection: WorkToolConnection) => void;
  onCancel: () => void;
}) {
  const [provider, setProvider] = useState<WorkToolProvider>(
    connection?.provider ?? "notion",
  );
  const [name, setName] = useState(connection?.name ?? "");
  const secretInput = useRef<HTMLInputElement>(null);
  const [id] = useState(() => connection?.public_id ?? createIdempotencyKey());
  const pending = useRef<RetryIdentity>(undefined);
  const save = useProviderAction();
  function submit(event: FormEvent) {
    event.preventDefault();
    if (save.busy || !name.trim()) return;
    const secret = secretInput.current?.value.trim();
    if (!connection && !secret) return;
    const context = captureRequestContext();
    const input = {
      id,
      provider,
      name: name.trim(),
      expected_revision: connection?.revision ?? 0,
      ...(secret ? { api_key: secret } : {}),
    };
    void save.run(
      async () => {
        pending.current = await retryIdentity(input, pending.current);
        context.assertCurrent();
        save.assertActive();
        return workToolsApi.save(input, pending.current.key);
      },
      (value) => {
        if (secretInput.current) secretInput.current.value = "";
        pending.current = undefined;
        onSaved(value);
      },
    );
  }
  return (
    <form
      className="space-y-5 rounded-lg border p-6"
      onSubmit={submit}
      autoComplete="off"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">
          {connection ? "Modifier la connexion" : "Ajouter un outil de travail"}
        </h2>
        <Button
          type="button"
          variant="ghost"
          disabled={save.busy}
          onClick={onCancel}
        >
          Fermer
        </Button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <div>
          <label
            htmlFor="work-tool-provider"
            className="mb-2 block text-sm font-medium"
          >
            Outil
          </label>
          <select
            id="work-tool-provider"
            value={provider}
            disabled={Boolean(connection) || save.busy}
            onChange={(event) =>
              setProvider(event.target.value as WorkToolProvider)
            }
            className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
          >
            {(["notion", "linear", "github"] as const).map((item) => (
              <option key={item} value={item}>
                {providerLabels[item]}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label
            htmlFor="work-tool-name"
            className="mb-2 block text-sm font-medium"
          >
            Nom pour l’équipe
          </label>
          <Input
            id="work-tool-name"
            value={name}
            maxLength={120}
            required
            disabled={save.busy}
            onChange={(event) => setName(event.target.value)}
            placeholder="Documentation de l’entreprise"
          />
        </div>
      </div>
      <div>
        <label
          htmlFor="work-tool-key"
          className="mb-2 block text-sm font-medium"
        >
          {connection ? "Nouvelle clé d’accès (facultatif)" : "Clé d’accès"}
        </label>
        <Input
          id="work-tool-key"
          type="password"
          ref={secretInput}
          autoComplete="new-password"
          minLength={8}
          maxLength={4096}
          required={!connection}
          disabled={save.busy}
        />
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          La clé est envoyée au serveur pour être conservée chiffrée. Elle ne
          sera pas affichée après enregistrement.{" "}
          {connection
            ? "Laissez ce champ vide pour garder la clé actuelle."
            : "Utilisez une clé dédiée aux destinations que votre équipe souhaite utiliser."}
        </p>
      </div>
      {connection && !connection.enabled ? (
        <p className="text-sm text-amber-900">
          L’enregistrement réactivera cette connexion.
        </p>
      ) : null}
      <Button type="submit" disabled={save.busy || !name.trim()}>
        {save.busy ? "Enregistrement…" : "Enregistrer la connexion"}
      </Button>
      {save.error ? (
        <p role="alert" className="text-sm text-destructive">
          {save.error}
        </p>
      ) : null}
    </form>
  );
}
