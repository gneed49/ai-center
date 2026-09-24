import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { Plus } from "lucide-react";
import { companyApi } from "@/api/company";
import { useAuth } from "@/auth/auth-context";
import { workToolsApi } from "@/api/work-tools";
import type {
  WorkToolConnection,
  WorkToolSettings,
} from "@/api/work-tool-types";
import { WorkToolConnectionForm } from "@/components/work-tools/connection-form";
import { WorkToolConnectionCard } from "@/components/work-tools/connection-card";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";

export function WorkToolsPage() {
  const auth = useAuth();
  return (
    <ScopedWorkTools
      key={`${auth.session?.user.id ?? "local"}:${auth.workspaceId ?? "local"}`}
    />
  );
}
function ScopedWorkTools() {
  const cache = useQueryClient();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const settings = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const [editing, setEditing] = useState<WorkToolConnection | "new" | null>(
    null,
  );
  const [notice, setNotice] = useState<string | null>(null);
  function changed(value: WorkToolConnection) {
    cache.setQueryData<WorkToolSettings>(["work-tools"], (current) =>
      current
        ? {
            ...current,
            connections: [
              ...current.connections.filter(
                (connection) => connection.public_id !== value.public_id,
              ),
              value,
            ],
          }
        : current,
    );
    void cache.invalidateQueries({ queryKey: ["work-tools"] });
    setEditing(null);
    setNotice(
      value.enabled
        ? "Connexion enregistrée. Aucune publication n’a été lancée."
        : "Connexion désactivée.",
    );
  }
  if (settings.isPending || company.isPending)
    return <LoadingState label="Chargement des outils de l’équipe…" />;
  if (settings.isError || company.isError)
    return (
      <ErrorState
        error={(settings.error ?? company.error)!}
        retry={() => {
          void settings.refetch();
          void company.refetch();
        }}
      />
    );
  const owner = company.data.workspace.role === "owner";
  return (
    <div className="space-y-7">
      <PageHeader
        title="Outils de l’équipe"
        eyebrow={company.data.workspace.name}
        description="Reliez les destinations où votre équipe veut publier ses livrables validés."
        actions={
          <Button asChild variant="outline">
            <Link to="/settings/ai">Réglages IA personnels</Link>
          </Button>
        }
      />
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="max-w-3xl text-sm leading-6 text-muted-foreground">
          Les connexions sont communes à l’entreprise. Une publication crée une
          page ou un ticket à votre demande ; les modifications ultérieures dans
          l’outil ne sont pas écrasées automatiquement.
        </p>
        <Button asChild variant="outline">
          <Link to="/settings/destinations">Choisir les destinations</Link>
        </Button>
      </div>
      {!settings.data.storage_available ? (
        <p
          role="alert"
          className="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900"
        >
          L’enregistrement sécurisé des clés n’est pas disponible. Le
          propriétaire de l’instance doit configurer ce service avant de relier
          un outil.
        </p>
      ) : null}
      {settings.data.usage && settings.data.limits ? (
        <section
          className="rounded-lg border p-5"
          aria-label="Activité et limites des publications"
        >
          <h2 className="font-semibold">Activité de l’entreprise</h2>
          <dl className="mt-4 grid gap-4 text-sm sm:grid-cols-3">
            <div>
              <dt className="text-muted-foreground">File en cours</dt>
              <dd className="mt-1 font-medium">
                {settings.data.usage.queued} en attente ·{" "}
                {settings.data.usage.processing} en traitement
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">
                Publications sur la dernière heure
              </dt>
              <dd className="mt-1 font-medium">
                {settings.data.usage.publications_last_hour} /{" "}
                {settings.data.limits.max_publications_per_hour}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Résultats à vérifier</dt>
              <dd className="mt-1 font-medium">
                {settings.data.usage.needs_review}
              </dd>
            </div>
          </dl>
          <p className="mt-4 text-xs text-muted-foreground">
            Les lectures distantes sont limitées à{" "}
            {settings.data.limits.max_remote_reads_per_hour} par heure.{" "}
            {settings.data.usage.known_cost_usd === null
              ? "Ces services ne fournissent pas de coût réel vérifié à AI Center."
              : `Coût connu communiqué : ${settings.data.usage.known_cost_usd} USD.`}
          </p>
        </section>
      ) : null}
      {!owner ? (
        <p className="text-sm text-muted-foreground">
          Le propriétaire peut ajouter, vérifier ou désactiver les connexions.
          Les collaborateurs peuvent publier un livrable validé vers une
          destination autorisée.
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="text-sm text-emerald-800">
          {notice}
        </p>
      ) : null}
      {editing && owner && settings.data.storage_available ? (
        <WorkToolConnectionForm
          key={
            editing === "new"
              ? "new"
              : `${editing.public_id}:${editing.revision}`
          }
          connection={editing === "new" ? undefined : editing}
          onSaved={changed}
          onCancel={() => setEditing(null)}
        />
      ) : owner ? (
        <Button
          disabled={!settings.data.storage_available}
          onClick={() => {
            setNotice(null);
            setEditing("new");
          }}
        >
          <Plus />
          Ajouter une connexion
        </Button>
      ) : null}
      {settings.data.connections.length ? (
        <div className="grid gap-5 xl:grid-cols-2">
          {settings.data.connections.map((connection) => (
            <WorkToolConnectionCard
              key={`${connection.public_id}:${connection.revision}`}
              connection={connection}
              owner={owner}
              storage={settings.data.storage_available}
              onEdit={() => {
                setNotice(null);
                setEditing(connection);
              }}
              onChanged={changed}
            />
          ))}
        </div>
      ) : (
        <EmptyState
          title="Aucun outil relié"
          description="Ajoutez une connexion Notion, Linear ou GitHub, puis choisissez une destination pour vos livrables."
        />
      )}
    </div>
  );
}
