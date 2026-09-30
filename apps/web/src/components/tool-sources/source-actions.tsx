import { useState } from "react";
import type {
  ToolSourceReference,
  SourceAction,
} from "@/api/tool-source-types";
import type { WorkToolSettings } from "@/api/work-tool-types";
import { Button } from "@/components/ui/button";
import { SourceCommandStatus } from "./source-command-status";
import { useSourceCommand } from "./use-source-command";
export function SourceActions(props: {
  reference: ToolSourceReference;
  settings: WorkToolSettings;
  canEdit: boolean;
  owner: boolean;
  companyName: string;
  scopeName: string;
}) {
  return (
    <section className="space-y-4" aria-label="Actions sur la source">
      {(["refresh", "detach", "rebind"] as const).map((action) => (
        <SourceActionForm
          key={`${action}:${props.reference.public_id}`}
          action={action}
          {...props}
        />
      ))}
    </section>
  );
}
function SourceActionForm({
  action,
  reference,
  settings,
  canEdit,
  owner,
  companyName,
  scopeName,
}: Parameters<typeof SourceActions>[0] & {
  action: Exclude<SourceAction, "attach">;
}) {
  const command = useSourceCommand(
    reference.project_id,
    action,
    reference.public_id,
  );
  const [open, setOpen] = useState(false);
  const [connectionId, setConnectionId] = useState("");
  const [sharing, setSharing] = useState(false);
  const connections = settings.connections.filter(
    (c) =>
      c.provider === reference.provider &&
      c.enabled &&
      c.allow_existing_reads === true,
  );
  const connection = connections.find(
    (c) =>
      c.public_id ===
      (action === "rebind" ? connectionId : reference.connection_id),
  );
  const allowed = canEdit && (action !== "rebind" || owner);
  const readCapable =
    settings.storage_available &&
    settings.capabilities.some(
      (c) => c.provider === reference.provider && c.read_existing,
    );
  const canSend =
    allowed &&
    (action === "rebind"
      ? Boolean(readCapable && connection && sharing)
      : reference.status === "active" &&
        (action === "detach" || Boolean(readCapable && connection)));
  const label =
    action === "refresh"
      ? "Vérifier dans l’outil"
      : action === "detach"
        ? "Retirer du contexte"
        : reference.status === "detached"
          ? "Réactiver la source"
          : "Changer la connexion";
  function submit() {
    if (!canSend || command.saved) return;
    const expected = {
      expected_revision: reference.revision,
      expected_observation_id: reference.current_observation_id,
    };
    if (action === "rebind" && connection)
      command.submit({
        action,
        referenceId: reference.public_id,
        input: {
          ...expected,
          connection_id: connection.public_id,
          expected_connection_revision: connection.revision,
          confirm_scope_sharing: true,
        },
      });
    else if (action !== "rebind")
      command.submit({
        action,
        referenceId: reference.public_id,
        input: expected,
      });
    setOpen(false);
  }
  return (
    <div className="space-y-3">
      <SourceCommandStatus command={command} canAct={allowed} />
      {!command.saved &&
      allowed &&
      (reference.status === "active" || action === "rebind") ? (
        <>
          <Button
            variant={action === "refresh" ? "default" : "outline"}
            disabled={action === "refresh" && !canSend}
            onClick={() => (action === "refresh" ? submit() : setOpen(!open))}
          >
            {label}
          </Button>
          {open ? (
            <div
              className="space-y-4 rounded-lg border p-4"
              role="region"
              aria-label={label}
            >
              {action === "detach" ? (
                <p className="text-sm">
                  Cette source ne sera plus proposée aux agents dans ce
                  contexte. Son historique et les documents qui la citent
                  restent consultables. Le document dans l’outil reste inchangé.
                </p>
              ) : (
                <>
                  <label className="block text-sm">
                    Connexion à vérifier
                    <select
                      className="mt-1 min-h-11 w-full rounded border bg-white px-3"
                      value={connectionId}
                      onChange={(e) => {
                        setConnectionId(e.target.value);
                        setSharing(false);
                      }}
                    >
                      <option value="">Choisir une connexion</option>
                      {connections.map((c) => (
                        <option key={c.public_id} value={c.public_id}>
                          {c.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  {!connections.length ? (
                    <p className="text-sm">
                      Autorisez d’abord la lecture dans les réglages de
                      connexion.
                    </p>
                  ) : null}
                  <label className="flex items-start gap-3 text-sm leading-6">
                    <input
                      className="mt-1 size-4 shrink-0"
                      type="checkbox"
                      checked={sharing}
                      onChange={(e) => setSharing(e.target.checked)}
                    />
                    Je partage le contenu lu avec les membres autorisés de{" "}
                    {companyName}, dans le contexte « {scopeName} ».
                  </label>
                  <p className="text-sm text-muted-foreground">
                    Une nouvelle lecture doit réussir avant que cette connexion
                    ou la réactivation prenne effet.
                  </p>
                </>
              )}
              <div className="flex gap-3">
                <Button disabled={!canSend} onClick={submit}>
                  {action === "detach"
                    ? "Confirmer le retrait"
                    : "Vérifier et réactiver"}
                </Button>
                <Button variant="ghost" onClick={() => setOpen(false)}>
                  Annuler
                </Button>
              </div>
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
