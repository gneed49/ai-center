import { useId, useRef, useState, type SubmitEvent } from "react";
import { providersApi } from "@/api/providers";
import { captureRequestContext } from "@/api/request-context";
import type {
  Provider,
  ProviderConnection,
  ProviderModel,
} from "@/api/provider-types";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { retryIdentity, type RetryIdentity } from "@/lib/provider-settings";
import { useProviderAction } from "./use-provider-action";

export function ConnectionForm({
  provider,
  connection,
  storageAvailable,
  models = [],
  onSaved,
  onCancel,
}: {
  provider: Provider;
  connection?: ProviderConnection;
  storageAvailable: boolean;
  models?: ProviderModel[];
  onSaved: (connection: ProviderConnection) => void;
  onCancel?: () => void;
}) {
  const prefix = useId();
  const secretInput = useRef<HTMLInputElement>(null);
  const [connectionId] = useState(() => connection?.id ?? crypto.randomUUID());
  const [name, setName] = useState(connection?.name ?? "");
  const [model, setModel] = useState(connection?.model ?? "");
  const retry = useRef<RetryIdentity>(undefined);
  const action = useProviderAction();
  const hasKey = provider.auth_method === "api_key";
  const disabled =
    action.busy || !provider.available || (hasKey && !storageAvailable);

  function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (disabled) return;
    const context = captureRequestContext();
    // The key lives only in this input and this request's local stack.
    const apiKey = secretInput.current?.value.trim();
    const input = {
      name: name.trim(),
      model: model.trim(),
      ...(apiKey ? { api_key: apiKey } : {}),
    };
    void action.run(
      async () => {
        retry.current = await retryIdentity(
          { ...input, id: connectionId, provider: provider.id },
          retry.current,
        );
        context.assertCurrent();
        action.assertActive();
        return connection
          ? providersApi.update(connectionId, input, retry.current.key)
          : providersApi.create(
              { ...input, id: connectionId, provider: provider.id },
              retry.current.key,
            );
      },
      (saved) => {
        if (secretInput.current) secretInput.current.value = "";
        retry.current = undefined;
        onSaved(saved);
      },
    );
  }

  return (
    <form
      onSubmit={submit}
      aria-label={
        connection ? `Modifier ${connection.name}` : "Nouvelle connexion IA"
      }
      autoComplete="off"
    >
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor={`${prefix}-name`}>
            Nom de la connexion
          </FieldLabel>
          <Input
            id={`${prefix}-name`}
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Ex. Personnel, Travail"
            required
            maxLength={120}
            disabled={action.busy}
          />
          <FieldDescription>
            Plusieurs connexions du même fournisseur peuvent coexister.
          </FieldDescription>
        </Field>
        <Field>
          <FieldLabel htmlFor={`${prefix}-model`}>Modèle</FieldLabel>
          <Input
            id={`${prefix}-model`}
            value={model}
            onChange={(event) => setModel(event.target.value)}
            placeholder="Identifiant du modèle proposé par le fournisseur"
            required
            maxLength={200}
            list={models.length ? `${prefix}-models` : undefined}
            disabled={action.busy}
          />
          {models.length > 0 && (
            <datalist id={`${prefix}-models`}>
              {models.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </datalist>
          )}
          <FieldDescription>
            {models.length
              ? "Choisissez un modèle découvert ou saisissez son identifiant."
              : "L’identifiant reste modifiable. Après enregistrement, vous pouvez consulter les modèles accessibles."}
          </FieldDescription>
        </Field>
        {hasKey && (
          <Field data-disabled={!storageAvailable}>
            <FieldLabel htmlFor={`${prefix}-key`}>
              {connection ? "Nouvelle clé API (facultatif)" : "Clé API"}
            </FieldLabel>
            <Input
              ref={secretInput}
              id={`${prefix}-key`}
              type="password"
              autoComplete="new-password"
              autoCapitalize="none"
              spellCheck={false}
              required={!connection}
              maxLength={4096}
              disabled={disabled}
            />
            <FieldDescription>
              {connection
                ? "Laissez vide pour conserver la clé actuelle. "
                : ""}
              La clé est chiffrée sur le serveur et effacée du formulaire après
              enregistrement.
            </FieldDescription>
          </Field>
        )}
        {!hasKey && (
          <FieldDescription>
            Enregistrez le profil, puis connectez votre abonnement avec le
            client officiel depuis sa fiche.
          </FieldDescription>
        )}
        {action.error && <FieldError>{action.error}</FieldError>}
        <Field orientation="horizontal">
          <Button type="submit" disabled={disabled}>
            {action.busy
              ? "Enregistrement…"
              : connection
                ? "Enregistrer les modifications"
                : "Ajouter la connexion"}
          </Button>
          {onCancel && (
            <Button
              type="button"
              variant="outline"
              onClick={onCancel}
              disabled={action.busy}
            >
              Annuler
            </Button>
          )}
        </Field>
      </FieldGroup>
    </form>
  );
}
