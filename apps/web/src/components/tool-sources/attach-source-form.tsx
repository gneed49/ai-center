import { useState, type FormEvent } from "react";
import { Link } from "react-router";
import type { WorkToolSettings } from "@/api/work-tool-types";
import type { SourceProvider } from "@/api/tool-source-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { normalizeSourceInput } from "./source-command";
import { useSourceCommand } from "./use-source-command";
import { SourceCommandStatus } from "./source-command-status";
export function AttachSourceForm({
  projectId,
  scopeName,
  companyName,
  canEdit,
  owner,
  settings,
}: {
  projectId: string;
  scopeName: string;
  companyName: string;
  canEdit: boolean;
  owner: boolean;
  settings: WorkToolSettings;
}) {
  const command = useSourceCommand(projectId, "attach", null);
  const [provider, setProvider] = useState<SourceProvider>("linear");
  const [connectionId, setConnectionId] = useState("");
  const [source, setSource] = useState("");
  const [sharing, setSharing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const supported = settings.capabilities.some(
    (c) => c.provider === provider && c.read_existing === true,
  );
  const connections = settings.connections.filter(
    (c) =>
      supported &&
      c.provider === provider &&
      c.enabled &&
      c.allow_existing_reads === true,
  );
  const selected = connections.find((c) => c.public_id === connectionId);
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!canEdit || !selected || !sharing || command.saved || command.busy)
      return;
    try {
      const normalized = normalizeSourceInput(provider, source);
      setError(null);
      command.submit({
        action: "attach",
        referenceId: null,
        input: {
          provider,
          connection_id: selected.public_id,
          expected_connection_revision: selected.revision,
          source: normalized,
          confirm_scope_sharing: true,
        },
      });
    } catch (failure) {
      setError(
        failure instanceof Error
          ? failure.message
          : "Ce lien n’est pas accepté.",
      );
    }
  }
  return (
    <section className="space-y-4" aria-label="Ajouter une source">
      <SourceCommandStatus command={command} canAct={canEdit} />
      {!command.saved && canEdit ? (
        <form
          onSubmit={submit}
          className="space-y-4 rounded-lg border p-5"
          autoComplete="off"
        >
          <h2 className="text-lg font-semibold">Ajouter une source</h2>
          <p className="text-sm text-muted-foreground">
            Lisez une page ou un ticket existant dans le contexte « {scopeName}{" "}
            ». Le document reste dans son outil ; aucune modification n’y sera
            faite.
          </p>
          <label className="block space-y-1 text-sm">
            Outil
            <select
              className="min-h-11 w-full rounded border bg-white px-3"
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value as SourceProvider);
                setConnectionId("");
                setSharing(false);
                setError(null);
              }}
            >
              <option value="linear">Linear</option>
              <option value="notion">Notion</option>
            </select>
          </label>
          {connections.length ? (
            <label className="block space-y-1 text-sm">
              Connexion autorisée
              <select
                className="min-h-11 w-full rounded border bg-white px-3"
                value={connectionId}
                onChange={(e) => {
                  setConnectionId(e.target.value);
                  setSharing(false);
                }}
                required
              >
                <option value="">Choisir une connexion</option>
                {connections.map((c) => (
                  <option key={c.public_id} value={c.public_id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <p className="rounded bg-amber-50 p-3 text-sm text-amber-950">
              {owner ? (
                <Link to="/settings/tools" className="underline">
                  Gérer les connexions pour autoriser cette lecture
                </Link>
              ) : (
                "Un propriétaire doit autoriser cette lecture dans Connexions."
              )}
            </p>
          )}
          <label className="block space-y-1 text-sm">
            Lien de la page ou du ticket
            <Input
              value={source}
              onChange={(e) => {
                setSource(e.target.value);
                setSharing(false);
              }}
              maxLength={2048}
              required
              placeholder={
                provider === "linear"
                  ? "Lien Linear ou identifiant comme PROD-42"
                  : "Lien d’une page Notion"
              }
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Un identifiant de page ou de ticket est également accepté. Aucun
            document lié, commentaire ou fichier joint n’est parcouru
            automatiquement.
          </p>
          <label className="flex items-start gap-3 rounded bg-slate-50 p-3 text-sm leading-6">
            <input
              type="checkbox"
              className="mt-1 size-4 shrink-0"
              checked={sharing}
              onChange={(e) => setSharing(e.target.checked)}
            />
            Je partage le contenu lu avec les membres autorisés de {companyName}{" "}
            dans AI Center, dans le contexte « {scopeName} ».
          </label>
          {error ? (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          ) : null}
          <Button
            disabled={
              !settings.storage_available ||
              !selected ||
              !sharing ||
              !source.trim()
            }
            type="submit"
          >
            Lire et ajouter la source
          </Button>
        </form>
      ) : null}
    </section>
  );
}
