import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, FileClock, GitCommitHorizontal } from "lucide-react";
import { Link, useParams } from "react-router";

import { api } from "@/api/client";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { formatDate, humanize, shortId } from "@/lib/format";

export function HistoryPage() {
  const { projectId = "" } = useParams();
  const history = useQuery({
    queryKey: ["history", projectId],
    queryFn: () => api.history(projectId),
  });
  if (history.isLoading) return <LoadingState />;
  if (history.error)
    return <ErrorState error={history.error} retry={() => history.refetch()} />;
  const items = history.data ?? [];
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Audit immuable"
        title="Historique du projet"
        description="Chaque confirmation, projection et décision laisse une trace avec son objet et son état."
        actions={
          <Button variant="outline" asChild>
            <Link to={`/projects/${projectId}`}>
              <ArrowLeft />
              Projet
            </Link>
          </Button>
        }
      />
      {items.length ? (
        <section className="border border-slate-200 bg-white">
          <div className="divide-y divide-slate-100">
            {items.map((event, index) => (
              <div
                key={event.public_id}
                className="grid gap-4 px-5 py-5 sm:grid-cols-[130px_32px_1fr_auto] sm:items-start sm:px-6"
              >
                <time className="text-xs text-slate-500">
                  {formatDate(event.occurred_at, true)}
                </time>
                <div className="relative hidden justify-center sm:flex">
                  <span className="grid size-7 place-items-center bg-indigo-50 text-indigo-600">
                    <GitCommitHorizontal className="size-3.5" />
                  </span>
                  {index < items.length - 1 ? (
                    <span className="absolute top-7 h-10 w-px bg-slate-200" />
                  ) : null}
                </div>
                <div>
                  <p className="text-sm font-semibold">
                    {humanize(event.action)}
                  </p>
                  <p className="mt-1 text-xs text-slate-500">
                    {humanize(event.object_kind)} ·{" "}
                    <span className="font-mono">
                      {shortId(event.object_public_id)}
                    </span>
                  </p>
                  {event.after_state ? (
                    <details className="mt-3">
                      <summary className="cursor-pointer text-xs font-medium text-indigo-600">
                        Voir l’état enregistré
                      </summary>
                      <pre className="mt-2 max-h-52 overflow-auto bg-slate-950 p-3 text-[10px] leading-5 text-slate-300">
                        {JSON.stringify(event.after_state, null, 2)}
                      </pre>
                    </details>
                  ) : null}
                </div>
                <span className="font-mono text-[10px] text-slate-500">
                  {shortId(event.public_id)}
                </span>
              </div>
            ))}
          </div>
        </section>
      ) : (
        <EmptyState
          title="Aucun événement"
          description="Les commits et décisions apparaîtront ici."
          action={<FileClock className="size-5 text-slate-400" />}
        />
      )}
    </div>
  );
}
