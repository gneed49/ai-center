import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { providersApi } from "@/api/providers";
import type {
  Provider,
  ProviderConnection,
  ProviderSelection,
  ProviderSettings,
} from "@/api/provider-types";
import { useAuth } from "@/auth/auth-context";
import { PageHeader, LoadingState, EmptyState } from "@/components/app/page";
import { ConnectionCard } from "@/components/providers/connection-card";
import { ConnectionForm } from "@/components/providers/connection-form";
import { useProviderAction } from "@/components/providers/use-provider-action";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { providerError } from "@/lib/provider-settings";

export function AiSettingsPage() {
  const auth = useAuth();
  const scope = `${auth.session?.user.id ?? "local"}:${auth.workspaceId ?? "local"}`;
  // Remount the entire transient form/login state when the identity changes.
  return <ScopedSettings key={scope} scope={scope} />;
}

function ScopedSettings({ scope }: { scope: string }) {
  const client = useQueryClient();
  const queryKey = ["ai-settings", scope];
  const settings = useQuery({
    queryKey,
    queryFn: ({ signal }) => providersApi.settings(signal),
  });
  const [notice, setNotice] = useState<string>();
  function saved(connection: ProviderConnection) {
    client.setQueryData<ProviderSettings>(queryKey, (current) =>
      current
        ? {
            ...current,
            connections: [
              ...current.connections.filter(
                (item) => item.id !== connection.id,
              ),
              connection,
            ],
          }
        : current,
    );
    setNotice(
      "Connexion enregistrée. Choisissez-la dans « Connexion utilisée » pour l’activer.",
    );
  }
  function deleted(id: string) {
    client.setQueryData<ProviderSettings>(queryKey, (current) =>
      current
        ? {
            ...current,
            connections: current.connections.filter((item) => item.id !== id),
            selection:
              current.selection.connection_id === id
                ? { mode: "deterministic", connection_id: null }
                : current.selection,
          }
        : current,
    );
    setNotice("Connexion supprimée.");
    void client.invalidateQueries({ queryKey });
  }
  const data = settings.data;
  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Personnel · par espace de travail"
        title="Réglages IA"
        description="Rassemblez vos fournisseurs et choisissez la connexion utilisée pour vos prochaines demandes."
      />
      {settings.isPending ? (
        <LoadingState label="Chargement des connexions IA…" />
      ) : settings.isError ? (
        <Alert variant="destructive">
          <AlertTitle>Réglages indisponibles</AlertTitle>
          <AlertDescription>{providerError(settings.error)}</AlertDescription>
          <Button variant="outline" onClick={() => void settings.refetch()}>
            Réessayer
          </Button>
        </Alert>
      ) : (
        data && (
          <>
            {notice && (
              <p role="status" className="text-sm text-muted-foreground">
                {notice}
              </p>
            )}
            {!data.storage.available && (
              <Alert>
                <AlertTitle>Enregistrement des clés indisponible</AlertTitle>
                <AlertDescription>
                  {data.storage.message ??
                    "Le stockage sécurisé doit être configuré sur le serveur."}
                </AlertDescription>
              </Alert>
            )}
            <Selection
              key={`${data.selection.mode}:${data.selection.connection_id}`}
              settings={data}
              onSelected={(selection) => {
                client.setQueryData<ProviderSettings>(queryKey, (current) =>
                  current ? { ...current, selection } : current,
                );
                setNotice(
                  "Connexion utilisée mise à jour pour vos prochaines demandes.",
                );
              }}
            />
            <section
              className="flex flex-col gap-4"
              aria-labelledby="connections-title"
            >
              <h2 id="connections-title" className="text-xl font-semibold">
                Mes connexions
              </h2>
              {data.connections.length ? (
                <div className="grid items-start gap-4 xl:grid-cols-2">
                  {data.connections.map((connection) => {
                    const provider = data.providers.find(
                      (item) => item.id === connection.provider,
                    ) ?? {
                      id: connection.provider,
                      name: connection.provider,
                      auth_method: "api_key" as const,
                      available: false,
                    };
                    return (
                      <ConnectionCard
                        key={connection.id}
                        connection={connection}
                        provider={provider}
                        storageAvailable={data.storage.available}
                        selected={
                          data.selection.mode === "connection" &&
                          data.selection.connection_id === connection.id
                        }
                        onSaved={saved}
                        onDeleted={deleted}
                      />
                    );
                  })}
                </div>
              ) : (
                <EmptyState
                  title="Aucune connexion personnelle"
                  description="Ajoutez une clé API ou connectez un abonnement disponible ci-dessous."
                />
              )}
            </section>
            <NewConnection
              providers={data.providers}
              storageAvailable={data.storage.available}
              onSaved={saved}
            />
            {data.providers.some((provider) => !provider.available) && (
              <Alert>
                <AlertTitle>Disponibilité des fournisseurs</AlertTitle>
                <AlertDescription>
                  <ul className="flex list-disc flex-col gap-2 pl-5">
                    {data.providers
                      .filter((provider) => !provider.available)
                      .map((provider) => (
                        <li key={provider.id}>
                          <strong>{provider.name} :</strong>{" "}
                          {provider.unavailable_reason ??
                            "Indisponible sur ce serveur."}
                        </li>
                      ))}
                  </ul>
                </AlertDescription>
              </Alert>
            )}
            <p className="text-sm text-muted-foreground">
              Les clés API et les abonnements utilisent les offres et limites de
              leur fournisseur. Les connexions par abonnement passent par un
              client officiel local ; leur disponibilité dépend de ce poste. Une
              connexion en échec ne provoque pas de bascule automatique vers une
              autre.
            </p>
          </>
        )
      )}
    </div>
  );
}

function Selection({
  settings,
  onSelected,
}: {
  settings: ProviderSettings;
  onSelected: (selection: ProviderSelection) => void;
}) {
  const initial =
    settings.selection.mode === "connection"
      ? (settings.selection.connection_id ?? "deterministic")
      : settings.selection.mode;
  const [selected, setSelected] = useState(initial);
  const action = useProviderAction();
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2>Connexion utilisée</h2>
        </CardTitle>
        <CardDescription>
          Ce choix s’applique à vos conversations, plans, évaluations de
          couverture et analyses automatiques.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            const selection: ProviderSelection =
              selected === "server_default" || selected === "deterministic"
                ? { mode: selected, connection_id: null }
                : { mode: "connection", connection_id: selected };
            void action.run(() => providersApi.select(selection), onSelected);
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="active-provider">
                Utiliser pour mes prochaines demandes
              </FieldLabel>
              <Select
                value={selected}
                onValueChange={setSelected}
                disabled={action.busy}
              >
                <SelectTrigger id="active-provider" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem value="server_default">
                      Configuration du serveur
                    </SelectItem>
                    <SelectItem value="deterministic">
                      Sans fournisseur · mode déterministe
                    </SelectItem>
                    {settings.connections.map((connection) => (
                      <SelectItem
                        key={connection.id}
                        value={connection.id}
                        disabled={
                          settings.providers.find(
                            (item) => item.id === connection.provider,
                          )?.available !== true
                        }
                      >
                        {connection.name} · {connection.model}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
              <FieldDescription>
                Le mode déterministe fonctionne sans requête vers un modèle
                externe. La configuration du serveur peut utiliser une API
                payante si elle a été configurée.
              </FieldDescription>
            </Field>
            {action.error && <FieldError>{action.error}</FieldError>}
            <Field>
              <Button
                type="submit"
                disabled={action.busy || selected === initial}
              >
                {action.busy ? "Application…" : "Utiliser cette connexion"}
              </Button>
            </Field>
          </FieldGroup>
        </form>
      </CardContent>
    </Card>
  );
}

function NewConnection({
  providers,
  storageAvailable,
  onSaved,
}: {
  providers: Provider[];
  storageAvailable: boolean;
  onSaved: (connection: ProviderConnection) => void;
}) {
  const [providerId, setProviderId] = useState(
    () =>
      providers.find((item) => item.available)?.id ?? providers[0]?.id ?? "",
  );
  const [revision, setRevision] = useState(0);
  const provider = providers.find((item) => item.id === providerId);
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2>Ajouter une connexion</h2>
        </CardTitle>
        <CardDescription>
          Vos identifiants restent personnels à cet espace de travail.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="new-provider">Fournisseur</FieldLabel>
            <Select value={providerId} onValueChange={setProviderId}>
              <SelectTrigger id="new-provider" className="w-full">
                <SelectValue placeholder="Choisissez un fournisseur" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {providers.map((item) => (
                    <SelectItem
                      key={item.id}
                      value={item.id}
                      disabled={!item.available}
                    >
                      {item.name} ·{" "}
                      {item.auth_method === "api_key"
                        ? "clé API"
                        : "abonnement"}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
        </FieldGroup>
        {provider && (
          <ConnectionForm
            key={`${providerId}:${revision}`}
            provider={provider}
            storageAvailable={storageAvailable}
            onSaved={(connection) => {
              setRevision((value) => value + 1);
              onSaved(connection);
            }}
          />
        )}
      </CardContent>
    </Card>
  );
}
