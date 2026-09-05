import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, CheckCircle2, FileText, Link2 } from "lucide-react";
import { Link, useParams } from "react-router";

import { api } from "@/api/client";
import {
  ErrorState,
  LoadingState,
  NotFoundState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { ExternalProofPanel } from "@/components/app/external-proof-panel";
import { Button } from "@/components/ui/button";
import { humanize, shortId } from "@/lib/format";

export function DeliverableDetailPage() {
  const { projectId = "", deliverableId = "" } = useParams();
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
  });
  const coverage = useQuery({
    queryKey: ["coverage", projectId],
    queryFn: () => api.coverage(projectId),
  });
  if (snapshot.isLoading) return <LoadingState />;
  if (snapshot.error)
    return (
      <ErrorState error={snapshot.error} retry={() => snapshot.refetch()} />
    );
  const item = snapshot.data?.deliverables.find(
    (deliverable) => deliverable.public_id === deliverableId,
  );
  if (!item)
    return (
      <NotFoundState
        title="Livrable introuvable dans ce projet"
        description="L’identifiant ne correspond à aucun livrable du projet ouvert."
      />
    );
  const coverageItems =
    coverage.data?.items.filter(
      (entry) => entry.deliverable_public_id === item.public_id,
    ) ?? [];
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow={`${humanize(item.deliverable_type)} · v${item.version}`}
        title={item.title}
        description={item.summary}
        actions={
          <Button variant="outline" asChild>
            <Link to={`/projects/${projectId}/deliverables`}>
              <ArrowLeft />
              Livrables
            </Link>
          </Button>
        }
      />
      {coverage.error ? (
        <ErrorState
          error={coverage.error}
          title="La couverture de ce livrable est indisponible"
          retry={() => coverage.refetch()}
        />
      ) : null}
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <article className="border border-slate-200 bg-white">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-5 py-4 sm:px-7">
            <div className="flex gap-2">
              <StatusPill status={item.status} />
              <StatusPill status={item.coverage_status} />
            </div>
            <span className="font-mono text-xs text-slate-500">
              {shortId(item.public_id)}
            </span>
          </div>
          <div className="space-y-8 p-5 sm:p-8">
            {Object.entries(item.content).map(([key, value]) => (
              <section key={key}>
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
                  {humanize(key)}
                </p>
                <Content value={value} />
              </section>
            ))}
          </div>
        </article>
        <aside className="space-y-4">
          <div className="border border-slate-200 bg-white p-5">
            <div className="flex items-center gap-2">
              <Link2 className="size-4 text-indigo-500" />
              <h2 className="font-semibold">Traçabilité</h2>
            </div>
            <dl className="mt-4 space-y-3 text-xs">
              <div className="flex justify-between">
                <dt className="text-slate-500">Contrat</dt>
                <dd className="font-medium">
                  {humanize(item.deliverable_type)}
                </dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-500">Version</dt>
                <dd className="font-medium">v{item.version}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-500">Couverture</dt>
                <dd>
                  <StatusPill status={item.coverage_status} />
                </dd>
              </div>
            </dl>
          </div>
          <div className="border border-slate-200 bg-white p-5">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="size-4 text-emerald-500" />
              <h2 className="font-semibold">Exigences reliées</h2>
            </div>
            {coverageItems.length ? (
              <div className="mt-4 space-y-3">
                {coverageItems.map((entry) => (
                  <div
                    key={entry.requirement_version_public_id}
                    className="border-l-2 border-amber-400 pl-3"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <p className="text-xs font-semibold">
                        {entry.requirement_title}
                      </p>
                      <StatusPill status={entry.status} />
                    </div>
                    <p className="mt-1 text-xs leading-5 text-slate-500">
                      {entry.explanation}
                    </p>
                  </div>
                ))}
              </div>
            ) : (
              <p className="mt-4 text-sm text-slate-500">
                Aucune exigence reliée à ce livrable.
              </p>
            )}
          </div>
        </aside>
      </div>
      <ExternalProofPanel
        projectId={projectId}
        deliverableId={item.public_id}
        coverageItems={coverageItems}
      />
    </div>
  );
}

function Content({ value }: { value: unknown }) {
  if (typeof value === "string")
    return <p className="mt-3 text-sm leading-7 text-slate-700">{value}</p>;
  if (Array.isArray(value))
    return (
      <div className="mt-3 space-y-2">
        {value.map((entry, index) => (
          <div
            key={index}
            className="flex gap-3 border-b border-slate-100 py-2 text-sm leading-6 text-slate-700"
          >
            <FileText className="mt-1 size-4 shrink-0 text-slate-400" />
            <div>
              {typeof entry === "string" ? entry : <Content value={entry} />}
            </div>
          </div>
        ))}
      </div>
    );
  if (value && typeof value === "object")
    return (
      <dl className="mt-3 space-y-3 border-l border-slate-200 pl-4">
        {Object.entries(value).map(([key, entry]) => (
          <div key={key}>
            <dt className="text-xs font-semibold text-slate-500">
              {humanize(key)}
            </dt>
            <dd>
              <Content value={entry} />
            </dd>
          </div>
        ))}
      </dl>
    );
  return <p className="mt-2 text-sm text-slate-500">—</p>;
}
