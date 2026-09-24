import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { workToolsApi } from "@/api/work-tools";
import type { Publication } from "@/api/work-tool-types";
import {
  publicationLabels,
  safeWorkToolUrl,
} from "@/lib/work-tool-publication";
import { providerLabels } from "@/components/artifacts/labels";
import { PublicationDetailPanel } from "./publication-detail";
import { Button } from "@/components/ui/button";
export function PublicationCard({
  publication,
  canEdit,
  currentVersionId,
  live = false,
}: {
  publication: Publication;
  canEdit: boolean;
  currentVersionId: string;
  live?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const detail = useQuery({
    queryKey: ["publication", publication.public_id],
    queryFn: () => workToolsApi.publication(publication.public_id),
    enabled:
      live ||
      publication.title == null ||
      publication.source_version_number == null,
    refetchInterval: (query) =>
      ["queued", "processing"].includes(
        query.state.data?.publication.status ?? publication.status,
      )
        ? 3000
        : false,
  });
  const job = detail.data?.publication ?? publication;
  const index = job.source_ticket_index ?? -1;
  const version = job.source_version_number;
  const url = safeWorkToolUrl(job.provider, job.external_url);
  return (
    <article
      className="min-w-0 space-y-3 rounded-lg border p-4"
      aria-label={job.title ?? "Publication conservée"}
    >
      <h4 className="break-words font-semibold">
        {job.title ?? "Publication conservée"}
      </h4>
      <p className="text-sm">
        {index >= 0 ? `Ticket ${index + 1}` : "Document complet"} ·{" "}
        {version ? `version ${version}` : "version source conservée"} ·{" "}
        {providerLabels[job.provider]}
      </p>
      <p className="text-sm font-medium" role="status">
        {publicationLabels[job.status]}
      </p>
      {job.version_id !== currentVersionId ? (
        <p className="text-sm text-amber-900">
          Une autre version est affichée. Cette publication conserve sa version
          source.
        </p>
      ) : null}
      <div className="flex flex-wrap gap-3 text-sm">
        <Link
          className="text-primary underline"
          to={`/artifacts/${job.artifact_id}?version=${job.version_id}${index >= 0 ? `&ticket=${index}` : ""}`}
        >
          Ouvrir la version source{index >= 0 ? ` · ticket ${index + 1}` : ""}
        </Link>
        {url ? (
          <a
            className="text-primary underline"
            href={url}
            target="_blank"
            rel="noopener noreferrer"
          >
            Ouvrir dans {providerLabels[job.provider]}
          </a>
        ) : null}
      </div>
      {detail.error ? (
        <p className="text-sm text-muted-foreground">
          L’état récent est indisponible ; le dernier reçu reste affiché.
        </p>
      ) : null}
      <Button
        variant="ghost"
        size="sm"
        aria-expanded={expanded}
        onClick={() => setExpanded(!expanded)}
      >
        Détails et observations
      </Button>
      {expanded ? (
        <PublicationDetailPanel
          publicationId={job.public_id}
          canEdit={canEdit}
        />
      ) : null}
    </article>
  );
}
