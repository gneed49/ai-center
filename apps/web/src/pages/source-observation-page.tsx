import { SourcePageFrame } from "@/components/tool-sources/source-page-frame";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";
import { toolSourcesApi } from "@/api/tool-sources";
import type { ObservationKind } from "@/api/tool-source-types";
import { companyApi } from "@/api/company";
import { workToolsApi } from "@/api/work-tools";
import { useAuth } from "@/auth/auth-context";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { PublicationDetailPanel } from "@/components/work-tools/publication-detail";
import { SourceObservationView } from "@/components/tool-sources/source-observation";
export function SourceObservationPage({ kind }: { kind: ObservationKind }) {
  const { observationId = "" } = useParams();
  const auth = useAuth();
  return (
    <ExactObservation
      key={`${auth.session?.user.id ?? "local"}:${auth.workspaceId ?? "local"}:${kind}:${observationId}`}
      kind={kind}
      observationId={observationId}
    />
  );
}
function ExactObservation({
  kind,
  observationId,
}: {
  kind: ObservationKind;
  observationId: string;
}) {
  const [publicationOpen, setPublicationOpen] = useState(false);
  const source = useQuery({
    queryKey: ["source-observation", kind, observationId],
    queryFn: () => toolSourcesApi.observation(kind, observationId),
    retry: false,
  });
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  if (source.error || company.error)
    return (
      <ErrorState
        error={(source.error ?? company.error)!}
        retry={() => {
          void source.refetch();
          void company.refetch();
        }}
      />
    );
  if (source.isPending || company.isPending)
    return <LoadingState label="Chargement de la lecture exacte…" />;
  const o = source.data.observation;
  const isCompany =
    o.source_project_public_id ===
    company.data.company_scope?.project_public_id;
  const project = company.data.projects.find(
    (p) => p.public_id === o.source_project_public_id,
  );
  const canEdit =
    company.data.workspace.role !== "viewer" &&
    (isCompany || project?.status === "active");
  return (
    <SourcePageFrame>
      <nav
        aria-label="Contexte de la lecture"
        className="flex flex-wrap gap-4 text-sm text-primary"
      >
        <Link
          to={
            isCompany
              ? "/sources"
              : `/projects/${o.source_project_public_id}/sources`
          }
        >
          {isCompany
            ? "Sources de la société"
            : (project?.name ?? "Projet source")}
        </Link>
        <Link to={`/graph?project=${o.source_project_public_id}`}>
          Voir la filiation dans le graphe
        </Link>
      </nav>
      <PageHeader
        title="Lecture exacte de la source"
        eyebrow={`Lecture enregistrée n°${o.version}`}
        description="Les citations conservent cette lecture, même si le contenu distant évolue."
        actions={
          <Button variant="outline" onClick={() => void source.refetch()}>
            Recharger l’état enregistré
          </Button>
        }
      />
      <SourceObservationView detail={source.data} />
      {o.publication_public_id ? (
        <section className="space-y-4">
          <Button
            variant="outline"
            aria-expanded={publicationOpen}
            onClick={() => setPublicationOpen(!publicationOpen)}
          >
            Voir la publication d’origine
          </Button>
          {publicationOpen ? (
            <PublicationOrigin
              publicationId={o.publication_public_id}
              canEdit={canEdit}
            />
          ) : null}
        </section>
      ) : null}
    </SourcePageFrame>
  );
}
function PublicationOrigin({
  publicationId,
  canEdit,
}: {
  publicationId: string;
  canEdit: boolean;
}) {
  const detail = useQuery({
    queryKey: ["publication", publicationId],
    queryFn: () => workToolsApi.publication(publicationId),
    retry: false,
  });
  if (detail.error) return <ErrorState error={detail.error} />;
  if (detail.isPending) return <LoadingState />;
  const job = detail.data.publication;
  const index = job.source_ticket_index ?? -1;
  return (
    <div className="space-y-3">
      <Link
        className="text-sm text-primary underline"
        to={`/artifacts/${job.artifact_id}?version=${job.version_id}${index >= 0 && index < 30 ? `&ticket=${index}` : ""}`}
      >
        Ouvrir le livrable à l’origine de cette publication
      </Link>
      <PublicationDetailPanel publicationId={publicationId} canEdit={canEdit} />
    </div>
  );
}
