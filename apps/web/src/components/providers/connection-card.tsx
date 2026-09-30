import { useState } from "react";
import { providersApi } from "@/api/providers";
import type {
  Provider,
  ProviderConnection,
  ProviderModel,
} from "@/api/provider-types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { FieldError } from "@/components/ui/field";
import { ConnectionForm } from "./connection-form";
import { SubscriptionPanel } from "./subscription-panel";
import { useProviderAction } from "./use-provider-action";

export function ConnectionCard({
  provider,
  connection,
  selected,
  storageAvailable,
  onSaved,
  onDeleted,
}: {
  provider: Provider;
  connection: ProviderConnection;
  selected: boolean;
  storageAvailable: boolean;
  onSaved: (connection: ProviderConnection) => void;
  onDeleted: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [models, setModels] = useState<ProviderModel[]>();
  const action = useProviderAction();
  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          <CardTitle>
            <h3>{connection.name}</h3>
          </CardTitle>
          {selected && <Badge>Utilisée</Badge>}
        </div>
        <CardDescription>
          {provider.name} ·{" "}
          {provider.auth_method === "api_key"
            ? "Clé API"
            : "Abonnement personnel"}
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <dl className="flex flex-col gap-1 text-sm">
          <dt className="text-muted-foreground">Modèle</dt>
          <dd className="break-all font-medium">{connection.model}</dd>
          {connection.key_hint && (
            <>
              <dt className="text-muted-foreground">Clé enregistrée</dt>
              <dd>{connection.key_hint}</dd>
            </>
          )}
        </dl>
        {!provider.available && (
          <p className="text-sm text-muted-foreground">
            {provider.unavailable_reason ??
              "Ce fournisseur est indisponible sur ce serveur."}
          </p>
        )}
        {provider.auth_method === "subscription" && (
          <SubscriptionPanel
            connectionId={connection.id}
            available={provider.available}
          />
        )}
        {models && (
          <p role="status" className="text-sm text-muted-foreground">
            Accès confirmé au catalogue : {models.length} modèle(s). Ouvrez «
            Modifier » pour choisir un identifiant. Cette vérification n’envoie
            aucun contenu de projet et ne teste pas la génération.
          </p>
        )}
        {editing && (
          <ConnectionForm
            key={connection.updated_at}
            provider={provider}
            connection={connection}
            storageAvailable={storageAvailable}
            models={models}
            onSaved={(saved) => {
              setEditing(false);
              setModels(undefined);
              onSaved(saved);
            }}
            onCancel={() => setEditing(false)}
          />
        )}
        {action.error && <FieldError>{action.error}</FieldError>}
      </CardContent>
      <CardFooter className="flex flex-wrap gap-2">
        {confirmDelete ? (
          <>
            <p className="w-full text-sm">
              Supprimer cette connexion ?{" "}
              {selected && "Le mode sans fournisseur sera sélectionné."}
            </p>
            <Button
              type="button"
              variant="destructive"
              disabled={action.busy}
              onClick={() =>
                void action.run(
                  () => providersApi.remove(connection.id),
                  () => onDeleted(connection.id),
                )
              }
            >
              Confirmer la suppression
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled={action.busy}
              onClick={() => setConfirmDelete(false)}
            >
              Conserver
            </Button>
          </>
        ) : (
          <>
            <Button
              type="button"
              variant="outline"
              disabled={action.busy || editing}
              onClick={() => setEditing(true)}
            >
              Modifier
            </Button>
            {provider.auth_method === "api_key" && (
              <Button
                type="button"
                variant="outline"
                disabled={action.busy || editing || !provider.available}
                onClick={() =>
                  void action.run(
                    () => providersApi.test(connection.id),
                    (result) => setModels(result.models),
                  )
                }
              >
                {action.busy
                  ? "Vérification…"
                  : "Vérifier l’accès et les modèles"}
              </Button>
            )}
            <Button
              type="button"
              variant="ghost"
              disabled={action.busy || editing}
              onClick={() => setConfirmDelete(true)}
            >
              Supprimer
            </Button>
          </>
        )}
      </CardFooter>
    </Card>
  );
}
