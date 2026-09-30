import { SourcePageFrame } from "@/components/tool-sources/source-page-frame";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";
import { companyApi } from "@/api/company";
import { toolSourcesApi } from "@/api/tool-sources";
import { workToolsApi } from "@/api/work-tools";
import { useAuth } from "@/auth/auth-context";
import { Button } from "@/components/ui/button";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  NotFoundState,
  PageHeader,
} from "@/components/app/page";
import { AttachSourceForm } from "@/components/tool-sources/attach-source-form";
import {
  coverageLabels,
  sourceDate,
  sourceProviderLabels,
  sourceTitle,
} from "@/components/tool-sources/source-labels";
export function ToolSourcesPage() {
  const { projectId } = useParams();
  const auth = useAuth();
  return (
    <ScopedSources
      key={`${auth.session?.user.id ?? "local"}:${auth.workspaceId ?? "local"}:${projectId ?? "company"}`}
      requestedProjectId={projectId}
    />
  );
}
function ScopedSources({
  requestedProjectId,
}: {
  requestedProjectId?: string;
}) {
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const settings = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const projectId =
    requestedProjectId ?? company.data?.company_scope?.project_public_id;
  const [provider, setProvider] = useState("");
  const [status, setStatus] = useState("active");
  const [cursors, setCursors] = useState<string[]>([]);
  const cursor = cursors.at(-1);
  const sources = useQuery({
    queryKey: ["tool-sources", projectId, provider, status, cursor],
    enabled: Boolean(projectId),
    queryFn: () =>
      toolSourcesApi.list(projectId!, { provider, status, cursor }),
    retry: false,
  });
  if (company.isPending || settings.isPending)
    return <LoadingState label="Chargement des sources…" />;
  if (company.error || settings.error)
    return (
      <ErrorState
        error={(company.error ?? settings.error)!}
        retry={() => {
          void company.refetch();
          void settings.refetch();
        }}
      />
    );
  if (!projectId)
    return (
      <EmptyState
        title="Préparez votre société"
        description="Le contexte général sera disponible après la configuration de la société."
        action={
          <Button asChild>
            <Link to="/company">Configurer la société</Link>
          </Button>
        }
      />
    );
  const isCompany = projectId === company.data.company_scope?.project_public_id;
  const project = company.data.projects.find((p) => p.public_id === projectId);
  if (!isCompany && !project)
    return <NotFoundState title="Projet introuvable dans cette société" />;
  const scopeName = isCompany
    ? "Contexte général de la société"
    : project!.name;
  const canEdit =
    company.data.workspace.role !== "viewer" &&
    (isCompany || project?.status === "active");
  return (
    <SourcePageFrame>
      <PageHeader
        title={isCompany ? "Sources de la société" : "Sources du projet"}
        eyebrow={scopeName}
        description="Retrouvez le contenu lu dans les outils de votre équipe, avec ses dates et ses limites."
        actions={
          <Button variant="outline" asChild>
            <Link to={`/graph?project=${projectId}`}>Voir le graphe</Link>
          </Button>
        }
      />
      <p className="text-sm text-muted-foreground">
        Ce contexte organise les sources partagées avec les membres autorisés de{" "}
        {company.data.workspace.name}. Il ne crée pas un espace privé
        supplémentaire. Les lectures sont vérifiées à votre demande.
      </p>
      <AttachSourceForm
        projectId={projectId}
        scopeName={scopeName}
        companyName={company.data.workspace.name}
        canEdit={canEdit}
        owner={company.data.workspace.role === "owner"}
        settings={settings.data}
      />
      {!canEdit ? (
        <p className="text-sm text-muted-foreground">
          Consultation uniquement : votre rôle ou l’état du projet ne permet pas
          de modifier ces sources.
        </p>
      ) : null}
      <section className="space-y-4" aria-label="Sources rattachées">
        <h2 className="text-lg font-semibold">Sources rattachées</h2>
        <div className="flex flex-wrap gap-4">
          <label className="text-sm">
            Outil affiché
            <select
              className="ml-2 min-h-11 rounded border bg-white px-3"
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value);
                setCursors([]);
              }}
            >
              <option value="">Tous les outils</option>
              <option value="linear">Linear</option>
              <option value="notion">Notion</option>
            </select>
          </label>
          <label className="text-sm">
            État affiché
            <select
              className="ml-2 min-h-11 rounded border bg-white px-3"
              value={status}
              onChange={(e) => {
                setStatus(e.target.value);
                setCursors([]);
              }}
            >
              <option value="active">Actives</option>
              <option value="detached">Retirées</option>
              <option value="all">Toutes</option>
            </select>
          </label>
        </div>
        {sources.isPending ? (
          <LoadingState />
        ) : sources.error ? (
          <ErrorState
            error={sources.error}
            retry={() => void sources.refetch()}
          />
        ) : (
          <>
            <p className="text-sm text-muted-foreground">
              {sources.data.total_count} source(s) correspondant aux filtres ·{" "}
              {sources.data.active_count} active(s) dans ce contexte
            </p>
            {!sources.data.items.length ? (
              <EmptyState
                title="Aucune source dans cette sélection"
                description="Ajoutez un document ou un ticket existant, ou changez les filtres pour retrouver les sources retirées."
              />
            ) : (
              <ul className="grid gap-4 lg:grid-cols-2">
                {sources.data.items.map(({ reference, observation: o }) => (
                  <li
                    key={reference.public_id}
                    className="min-w-0 space-y-2 break-words rounded-lg border p-5"
                  >
                    <p className="text-xs text-muted-foreground">
                      {sourceProviderLabels[o.provider]} ·{" "}
                      {reference.status === "detached"
                        ? "Retirée du contexte"
                        : "Source observée"}
                    </p>
                    <Link
                      className="block font-semibold text-primary"
                      to={`/sources/${reference.public_id}`}
                    >
                      {sourceTitle(o.title)}
                    </Link>
                    <p className="text-sm">{coverageLabels[o.coverage]}</p>
                    <p className="text-xs text-muted-foreground">
                      Contenu lu le {sourceDate(o.observed_at)}
                      <br />
                      Dernière vérification le{" "}
                      {sourceDate(reference.last_checked_at)}
                    </p>
                    {!o.freshness.eligible ? (
                      <p className="text-sm text-amber-900">
                        Cette lecture ne peut pas alimenter le contexte actuel.
                      </p>
                    ) : null}
                    {reference.last_check_status === "failed" ? (
                      <p className="text-sm text-amber-900">
                        La dernière vérification n’a pas abouti.
                      </p>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
            <div className="flex gap-3">
              <Button
                variant="outline"
                disabled={!cursors.length}
                onClick={() => setCursors((v) => v.slice(0, -1))}
              >
                Page précédente
              </Button>
              <Button
                variant="outline"
                disabled={!sources.data.next_cursor}
                onClick={() =>
                  setCursors((v) => [...v, sources.data.next_cursor!])
                }
              >
                Page suivante
              </Button>
            </div>
          </>
        )}
      </section>
    </SourcePageFrame>
  );
}
