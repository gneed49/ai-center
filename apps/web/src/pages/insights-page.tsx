import { useQuery } from "@tanstack/react-query";
import {
  ArrowRight,
  CheckCircle2,
  Inbox,
  ShieldAlert,
  TriangleAlert,
} from "lucide-react";
import { Link } from "react-router";

import { api } from "@/api/client";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { formatDate } from "@/lib/format";

export function InsightsPage() {
  const insights = useQuery({ queryKey: ["insights"], queryFn: api.insights });
  if (insights.isLoading)
    return <LoadingState label="Analyse de la Decision Inbox…" />;
  if (insights.error)
    return (
      <ErrorState error={insights.error} retry={() => insights.refetch()} />
    );
  const items = insights.data ?? [];
  const active = items.filter((item) =>
    ["candidate", "open", "accepted"].includes(item.status),
  );
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Decision Inbox"
        title="Les signaux qui méritent une décision."
        description="Le steward relie les contradictions et trous de preuve à leurs sources. Il ne corrige jamais silencieusement le projet."
      />
      <section className="grid gap-px border border-slate-200 bg-slate-200 sm:grid-cols-3">
        <Metric icon={Inbox} label="À traiter" value={active.length} />
        <Metric
          icon={ShieldAlert}
          label="Bloquants"
          value={active.filter((item) => item.severity === "blocking").length}
          tone="red"
        />
        <Metric
          icon={CheckCircle2}
          label="Résolus"
          value={items.filter((item) => item.status === "resolved").length}
          tone="emerald"
        />
      </section>
      {items.length ? (
        <section className="border border-slate-200 bg-white">
          <div className="grid grid-cols-[1fr_auto] border-b border-slate-200 px-5 py-3 text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500 sm:grid-cols-[140px_1fr_150px_120px] sm:px-6">
            <span>Signal</span>
            <span className="hidden sm:block">Explication</span>
            <span className="hidden sm:block">Confiance</span>
            <span>État</span>
          </div>
          <div className="divide-y divide-slate-100">
            {items.map((item) => (
              <Link
                key={item.public_id}
                to={`/insights/${item.public_id}`}
                className="group grid grid-cols-[1fr_auto] gap-4 px-5 py-5 hover:bg-slate-50 sm:grid-cols-[140px_1fr_150px_120px] sm:items-center sm:px-6"
              >
                <div>
                  <span className="flex items-center gap-2 text-xs font-semibold text-slate-700">
                    <TriangleAlert
                      className={
                        item.severity === "blocking"
                          ? "size-4 text-red-500"
                          : "size-4 text-amber-500"
                      }
                    />
                    {item.insight_type === "contradiction"
                      ? "Contradiction"
                      : "Trou de preuve"}
                  </span>
                  <span className="mt-1 block text-[10px] text-slate-400">
                    {formatDate(item.detected_at, true)}
                  </span>
                </div>
                <div className="col-span-2 sm:col-span-1">
                  <p className="text-sm font-semibold">{item.title}</p>
                  <p className="mt-1 line-clamp-2 text-sm leading-5 text-slate-500">
                    {item.explanation}
                  </p>
                </div>
                <div className="hidden sm:block">
                  <strong className="text-lg">
                    {Math.round(item.confidence * 100)}%
                  </strong>
                  <span className="block text-[10px] text-slate-400">
                    confiance
                  </span>
                </div>
                <div className="flex items-center justify-end gap-3">
                  <StatusPill status={item.status} />
                  <ArrowRight className="size-4 text-slate-400 transition group-hover:translate-x-1" />
                </div>
              </Link>
            ))}
          </div>
        </section>
      ) : (
        <EmptyState
          title="Aucun signal"
          description="La Decision Inbox se remplira à mesure que le steward analysera les commits et la couverture."
        />
      )}
    </div>
  );
}
function Metric({
  icon: Icon,
  label,
  value,
  tone,
}: {
  icon: typeof Inbox;
  label: string;
  value: number;
  tone?: "red" | "emerald";
}) {
  return (
    <div className="flex items-center justify-between bg-white p-5">
      <div>
        <p className="text-xs text-slate-500">{label}</p>
        <strong className="mt-2 block text-3xl">{value}</strong>
      </div>
      <Icon
        className={
          tone === "red"
            ? "size-6 text-red-500"
            : tone === "emerald"
              ? "size-6 text-emerald-500"
              : "size-6 text-indigo-500"
        }
      />
    </div>
  );
}
