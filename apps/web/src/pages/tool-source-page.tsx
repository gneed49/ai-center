import { SourcePageFrame } from "@/components/tool-sources/source-page-frame";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";
import { companyApi } from "@/api/company";
import { toolSourcesApi } from "@/api/tool-sources";
import { workToolsApi } from "@/api/work-tools";
import { useAuth } from "@/auth/auth-context";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { SourceObservationView } from "@/components/tool-sources/source-observation";
import { SourceHistory } from "@/components/tool-sources/source-history";
import { SourceActions } from "@/components/tool-sources/source-actions";
import { sourceTitle } from "@/components/tool-sources/source-labels";
export function ToolSourcePage() {
  const { referenceId = "" } = useParams();
  const auth = useAuth();
  return (
    <ScopedSource
      key={`${auth.session?.user.id ?? "local"}:${auth.workspaceId ?? "local"}:${referenceId}`}
      referenceId={referenceId}
    />
  );
}
function ScopedSource({ referenceId }: { referenceId: string }) {
  const source = useQuery({
    queryKey: ["tool-source", referenceId],
    queryFn: () => toolSourcesApi.source(referenceId),
    retry: false,
  });
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const settings = useQuery({
    queryKey: ["work-tools"],
    queryFn: workToolsApi.settings,
  });
  const observationId = source.data?.reference.current_observation_id;
  const observation = useQuery({
    queryKey: ["source-observation", "tool_source_observation", observationId],
    enabled: Boolean(observationId) && !source.error,
    queryFn: () =>
      toolSourcesApi.observation("tool_source_observation", observationId!),
    retry: false,
  });
  if (source.error || company.error || settings.error)
    return (
      <ErrorState
        error={(source.error ?? company.error ?? settings.error)!}
        retry={() => {
          void source.refetch();
          void company.refetch();
          void settings.refetch();
        }}
      />
    );
  if (source.isPending || company.isPending || settings.isPending)
    return <LoadingState />;
  const ref = source.data.reference;
  const isCompany =
    ref.project_id === company.data.company_scope?.project_public_id;
  const project = company.data.projects.find(
    (p) => p.public_id === ref.project_id,
  );
  const name = isCompany
    ? "Contexte général de la société"
    : (project?.name ?? "Projet source");
  const canEdit =
    company.data.workspace.role !== "viewer" &&
    (isCompany || project?.status === "active");
  return (
    <SourcePageFrame>
      <nav
        className="flex flex-wrap gap-4 text-sm text-primary"
        aria-label="Contexte de la source"
      >
        <Link
          to={isCompany ? "/sources" : `/projects/${ref.project_id}/sources`}
        >
          Sources · {name}
        </Link>
        <Link to={`/graph?project=${ref.project_id}`}>Graphe du contexte</Link>
      </nav>
      <PageHeader
        title={sourceTitle(source.data.observation.title)}
        eyebrow={name}
        description="Le contenu conserve la date de sa lecture. Une vérification explicite peut révéler une modification dans l’outil."
      />
      {observation.error ? (
        <ErrorState
          error={observation.error}
          retry={() => void observation.refetch()}
        />
      ) : observation.isPending ? (
        <LoadingState label="Chargement du contenu lu…" />
      ) : (
        <SourceObservationView detail={observation.data} />
      )}
      <SourceActions
        reference={ref}
        settings={settings.data}
        owner={company.data.workspace.role === "owner"}
        canEdit={canEdit}
        companyName={company.data.workspace.name}
        scopeName={name}
      />
      <SourceHistory key={referenceId} referenceId={referenceId} />
    </SourcePageFrame>
  );
}
