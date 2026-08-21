import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  GitCompareArrows,
  Link2,
  ShieldAlert,
  X,
} from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { api } from "@/api/client";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { formatDate, humanize, shortId } from "@/lib/format";

export function InsightDetailPage() {
  const { insightId = "" } = useParams();
  const queryClient = useQueryClient();
  const [justification, setJustification] = useState("");
  const insight = useQuery({
    queryKey: ["insight", insightId],
    queryFn: () => api.insight(insightId),
  });
  const action = useMutation({
    mutationFn: (decision: "accept" | "dismiss" | "resolve") =>
      api.actOnInsight(insightId, decision, justification),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["insight", insightId] });
      queryClient.invalidateQueries({ queryKey: ["insights"] });
      setJustification("");
      toast.success("Décision auditée");
    },
    onError: (error) => toast.error(error.message),
  });
  if (insight.isLoading) return <LoadingState />;
  if (insight.error)
    return <ErrorState error={insight.error} retry={() => insight.refetch()} />;
  if (!insight.data) return null;
  const data = insight.data;
  const open = ["candidate", "open", "accepted"].includes(data.insight.status);
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Signal steward"
        title={data.insight.title}
        description={data.insight.explanation}
        actions={
          <Button variant="outline" asChild>
            <Link to="/insights">
              <ArrowLeft />
              Decision Inbox
            </Link>
          </Button>
        }
      />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_390px]">
        <div className="space-y-6">
          <section
            className={
              data.insight.severity === "blocking"
                ? "border border-red-200 bg-red-50 p-5 sm:p-7"
                : "border border-amber-200 bg-amber-50 p-5 sm:p-7"
            }
          >
            <div className="flex flex-wrap items-center gap-2">
              <StatusPill status={data.insight.severity} />
              <StatusPill status={data.insight.status} />
            </div>
            <div className="mt-6 grid gap-5 sm:grid-cols-3">
              <Fact
                label="Confiance"
                value={`${Math.round(data.insight.confidence * 100)}%`}
              />
              <Fact
                label="Détecté"
                value={formatDate(data.insight.detected_at, true)}
              />
              <Fact label="Type" value={humanize(data.insight.insight_type)} />
            </div>
            {data.insight.resolution_justification ? (
              <div className="mt-6 border-t border-current/10 pt-5">
                <p className="text-xs font-semibold uppercase tracking-[0.14em]">
                  Justification
                </p>
                <p className="mt-2 text-sm leading-6">
                  {data.insight.resolution_justification}
                </p>
              </div>
            ) : null}
          </section>
          <section className="border border-slate-200 bg-white">
            <div className="flex items-center gap-3 border-b border-slate-200 px-5 py-4 sm:px-6">
              <GitCompareArrows className="size-5 text-indigo-500" />
              <div>
                <p className="font-semibold">Sources mises en regard</p>
                <p className="text-xs text-slate-500">
                  Versions exactes, jamais les résumés courants
                </p>
              </div>
            </div>
            <div className="grid gap-px bg-slate-200 md:grid-cols-2">
              {data.sources.map((source) => (
                <article
                  key={`${source.source_role}-${source.object_public_id}`}
                  className="bg-white p-5 sm:p-6"
                >
                  <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-indigo-600">
                    {humanize(source.source_role)}
                  </p>
                  <h3 className="mt-3 font-semibold">
                    {source.version_title ?? "Source"}
                  </h3>
                  <p className="mt-2 text-sm leading-6 text-slate-600">
                    {source.version_statement}
                  </p>
                  <p className="mt-5 flex items-center gap-2 font-mono text-[10px] text-slate-400">
                    <Link2 className="size-3" />
                    {shortId(source.object_public_id)}
                  </p>
                </article>
              ))}
            </div>
          </section>
        </div>
        <aside className="border border-slate-200 bg-white p-5 sm:p-6">
          <div className="flex items-center gap-2">
            <ShieldAlert className="size-5 text-indigo-500" />
            <h2 className="font-semibold">Décision humaine</h2>
          </div>
          <p className="mt-2 text-sm leading-6 text-slate-600">
            Accepter le signal, l’écarter ou le déclarer résolu. La
            justification est inscrite dans l’audit.
          </p>
          {open ? (
            <>
              <Textarea
                value={justification}
                onChange={(event) => setJustification(event.target.value)}
                placeholder="Expliquez la décision…"
                className="mt-5 min-h-28"
              />
              <div className="mt-4 grid gap-2">
                <Button
                  disabled={!justification.trim() || action.isPending}
                  onClick={() => action.mutate("accept")}
                >
                  <Check />
                  Accepter le signal
                </Button>
                <Button
                  variant="outline"
                  disabled={!justification.trim() || action.isPending}
                  onClick={() => action.mutate("resolve")}
                >
                  Marquer résolu
                </Button>
                <Button
                  variant="ghost"
                  disabled={!justification.trim() || action.isPending}
                  onClick={() => action.mutate("dismiss")}
                >
                  <X />
                  Rejeter le signal
                </Button>
              </div>
            </>
          ) : (
            <div className="mt-5 border border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-800">
              Cette décision est close. Une nouvelle version du graphe pourra
              déclencher une réévaluation.
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] font-semibold uppercase tracking-[0.14em] opacity-60">
        {label}
      </p>
      <p className="mt-1 text-lg font-semibold">{value}</p>
    </div>
  );
}
