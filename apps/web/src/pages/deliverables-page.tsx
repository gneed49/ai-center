import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  FileCheck2,
  FileText,
  Plus,
  ShieldCheck,
} from "lucide-react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { api } from "@/api/client";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { formatDate, humanize } from "@/lib/format";

export function DeliverablesPage() {
  const { projectId = "" } = useParams();
  const queryClient = useQueryClient();
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
  });
  const coverage = useQuery({
    queryKey: ["coverage", projectId],
    queryFn: () => api.coverage(projectId),
  });
  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
    queryClient.invalidateQueries({ queryKey: ["coverage", projectId] });
  };
  const brief = useMutation({
    mutationFn: () => api.featureBrief(projectId),
    onSuccess: () => {
      refresh();
      toast.success("Feature Brief généré");
    },
    onError: (error) => toast.error(error.message),
  });
  const plan = useMutation({
    mutationFn: async () => {
      const techSession = snapshot.data?.sessions.find(
        (item) => item.node_key === "tech",
      );
      if (!techSession)
        throw new Error("Effectuez d’abord le handoff Produit → Tech.");
      return api.technicalPlan(projectId, techSession.public_id);
    },
    onSuccess: () => {
      refresh();
      toast.success("Technical Delivery Plan généré");
    },
    onError: (error) => toast.error(error.message),
  });
  if (snapshot.isLoading) return <LoadingState />;
  if (snapshot.error)
    return (
      <ErrorState error={snapshot.error} retry={() => snapshot.refetch()} />
    );
  if (!snapshot.data) return null;
  const data = snapshot.data;
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Projections contractuelles"
        title="Livrables et couverture"
        description="Chaque projection garde son contrat, ses sources exactes et son état de fraîcheur vis-à-vis du graphe."
        actions={
          <Button variant="outline" asChild>
            <Link to={`/projects/${projectId}`}>
              <ArrowLeft />
              Projet
            </Link>
          </Button>
        }
      />
      <section className="grid gap-px border border-slate-200 bg-slate-200 sm:grid-cols-4">
        <Metric label="Livrables" value={data.deliverables.length} />
        <Metric label="Exigences" value={coverage.data?.total ?? 0} />
        <Metric
          label="Couvertes"
          value={coverage.data?.covered ?? 0}
          tone="emerald"
        />
        <Metric
          label="Partielles"
          value={(coverage.data?.partial ?? 0) + (coverage.data?.missing ?? 0)}
          tone="amber"
        />
      </section>
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <section>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Registre des livrables</h2>
            <span className="text-xs text-slate-500">
              {data.deliverables.length} version(s)
            </span>
          </div>
          {data.deliverables.length ? (
            <div className="space-y-3">
              {data.deliverables.map((item) => (
                <Link
                  to={`/projects/${projectId}/deliverables/${item.public_id}`}
                  key={item.public_id}
                  className="group grid gap-4 border border-slate-200 bg-white p-5 transition hover:border-indigo-300 sm:grid-cols-[48px_1fr_auto] sm:items-center"
                >
                  <span className="grid size-12 place-items-center bg-slate-100 text-slate-600">
                    <FileText className="size-5" />
                  </span>
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="font-semibold">{item.title}</h3>
                      <StatusPill status={item.status} />
                      <StatusPill status={item.coverage_status} />
                    </div>
                    <p className="mt-2 text-sm text-slate-600">
                      {item.summary}
                    </p>
                    <p className="mt-2 font-mono text-[10px] text-slate-400">
                      {humanize(item.deliverable_type)} · v{item.version} ·{" "}
                      {formatDate(item.updated_at, true)}
                    </p>
                  </div>
                  <ArrowRight className="size-4 text-slate-400 transition group-hover:translate-x-1 group-hover:text-indigo-600" />
                </Link>
              ))}
            </div>
          ) : (
            <EmptyState
              title="Aucun livrable"
              description="Passez le gate Produit pour produire le premier Feature Brief."
            />
          )}
        </section>
        <aside className="space-y-4">
          <ActionCard
            icon={FileCheck2}
            title="Feature Brief"
            description="Compile le cadrage Produit selon son contrat obligatoire."
            action="Générer"
            pending={brief.isPending}
            onClick={() => brief.mutate()}
          />
          <ActionCard
            icon={ShieldCheck}
            title="Plan de livraison Tech"
            description="Relie architecture, étapes, risques et preuves aux exigences."
            action="Générer"
            pending={plan.isPending}
            onClick={() => plan.mutate()}
          />
          <div className="border border-slate-200 bg-[#11182b] p-5 text-white">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-300">
              Règle de fraîcheur
            </p>
            <p className="mt-3 text-sm leading-6 text-slate-300">
              Une nouvelle version du graphe marque automatiquement ContextPacks
              et livrables dépendants comme obsolètes.
            </p>
          </div>
        </aside>
      </div>
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone?: "emerald" | "amber";
}) {
  return (
    <div className="bg-white p-5">
      <p className="text-xs text-slate-500">{label}</p>
      <strong
        className={
          tone === "emerald"
            ? "mt-2 block text-3xl text-emerald-600"
            : tone === "amber"
              ? "mt-2 block text-3xl text-amber-600"
              : "mt-2 block text-3xl"
        }
      >
        {value}
      </strong>
    </div>
  );
}
function ActionCard({
  icon: Icon,
  title,
  description,
  action,
  pending,
  onClick,
}: {
  icon: typeof Plus;
  title: string;
  description: string;
  action: string;
  pending: boolean;
  onClick: () => void;
}) {
  return (
    <div className="border border-slate-200 bg-white p-5">
      <span className="grid size-9 place-items-center bg-indigo-50 text-indigo-600">
        <Icon className="size-4" />
      </span>
      <h3 className="mt-4 font-semibold">{title}</h3>
      <p className="mt-2 text-sm leading-6 text-slate-600">{description}</p>
      <Button
        className="mt-4 w-full"
        variant="outline"
        disabled={pending}
        onClick={onClick}
      >
        <Plus />
        {pending ? "Génération…" : action}
      </Button>
    </div>
  );
}
