import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Box, GitBranch, Plus, Sparkles } from "lucide-react";
import { Link } from "react-router";

import { api } from "@/api/client";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { formatDate } from "@/lib/format";

export function CenterPage() {
  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  if (projects.isLoading) return <LoadingState label="Ouverture du Center…" />;
  if (projects.error)
    return (
      <ErrorState error={projects.error} retry={() => projects.refetch()} />
    );
  const items = projects.data ?? [];

  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Workspace personnel"
        title="Un seul endroit pour garder le contexte vivant."
        description="Cadrez une intention, confirmez les connaissances utiles, puis transmettez un contexte vérifiable jusqu’au livrable et à sa preuve."
        actions={
          <Button asChild>
            <Link to="/projects/new">
              <Plus />
              Nouveau projet
            </Link>
          </Button>
        }
      />
      <section className="grid gap-px border border-slate-200 bg-slate-200 sm:grid-cols-3">
        <Metric
          icon={Box}
          label="Projets actifs"
          value={String(
            items.filter((item) => item.status === "active").length,
          ).padStart(2, "0")}
        />
        <Metric
          icon={GitBranch}
          label="Versions du graphe"
          value={String(
            items.reduce((sum, item) => sum + item.graph_version, 0),
          ).padStart(2, "0")}
        />
        <Metric
          icon={Sparkles}
          label="Moteur"
          value="Actif"
          detail="Agent server-side"
        />
      </section>
      <section>
        <div className="mb-4 flex items-end justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
              Portfolio
            </p>
            <h2 className="mt-1 text-xl font-semibold tracking-[-0.025em]">
              Projets récents
            </h2>
          </div>
          <p className="hidden text-xs text-slate-500 sm:block">
            {items.length} projet{items.length > 1 ? "s" : ""}
          </p>
        </div>
        {items.length === 0 ? (
          <EmptyState
            title="Le Center est prêt"
            description="Créez votre premier projet pour instancier les scopes Produit et Tech."
            action={
              <Button asChild>
                <Link to="/projects/new">Créer un projet</Link>
              </Button>
            }
          />
        ) : (
          <div className="grid gap-4 xl:grid-cols-2">
            {items.map((project) => (
              <Link
                key={project.public_id}
                to={`/projects/${project.public_id}`}
                className="group border border-slate-200 bg-white p-5 shadow-[0_1px_2px_rgba(15,23,42,0.03)] transition hover:border-indigo-300 hover:shadow-[0_12px_32px_rgba(30,41,59,0.08)] sm:p-6"
              >
                <div className="flex items-start justify-between gap-4">
                  <StatusPill status={project.status} label="Actif" />
                  <span className="text-xs text-slate-400">
                    mis à jour {formatDate(project.updated_at)}
                  </span>
                </div>
                <h3 className="mt-6 text-2xl font-semibold tracking-[-0.03em] text-slate-950">
                  {project.name}
                </h3>
                <p className="mt-2 line-clamp-2 min-h-12 text-sm leading-6 text-slate-600">
                  {project.objective ||
                    "Objectif à préciser dans la session Produit."}
                </p>
                <div className="mt-6 flex items-center justify-between border-t border-slate-100 pt-4">
                  <span className="font-mono text-xs text-slate-500">
                    graphe v{project.graph_version}
                  </span>
                  <span className="flex items-center gap-2 text-sm font-semibold text-indigo-600">
                    Ouvrir{" "}
                    <ArrowRight className="size-4 transition-transform group-hover:translate-x-1" />
                  </span>
                </div>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function Metric({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Box;
  label: string;
  value: string;
  detail?: string;
}) {
  return (
    <div className="bg-white p-5 sm:p-6">
      <div className="flex items-center gap-2 text-xs font-medium text-slate-500">
        <Icon className="size-4 text-indigo-500" />
        {label}
      </div>
      <div className="mt-4 flex items-end gap-3">
        <strong className="text-3xl font-semibold tracking-[-0.04em]">
          {value}
        </strong>
        {detail ? (
          <span className="pb-1 text-xs text-slate-400">{detail}</span>
        ) : null}
      </div>
    </div>
  );
}
