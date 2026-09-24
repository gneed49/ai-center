import { useQuery } from "@tanstack/react-query";
import {
  ArrowRight,
  Bot,
  FolderKanban,
  Network,
  Plus,
  Settings2,
} from "lucide-react";
import { Link, useLocation } from "react-router";

import { api } from "@/api/client";
import { companyApi } from "@/api/company";
import { EmptyState, ErrorState, LoadingState } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { formatDate } from "@/lib/format";

export function CenterPage() {
  const { pathname } = useLocation();
  const listOnly = pathname === "/projects";
  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  if (projects.isPending)
    return <LoadingState label="Chargement de vos projets…" />;
  if (projects.isError)
    return (
      <ErrorState
        error={projects.error}
        retry={() => void projects.refetch()}
      />
    );
  const items = projects.data;

  return (
    <div className="space-y-8">
      <header className="flex flex-col gap-5 sm:flex-row sm:items-start sm:justify-between">
        <div className="max-w-3xl">
          <h1 className="text-3xl font-semibold tracking-tight">
            {listOnly
              ? "Les projets de votre entreprise"
              : "Le contexte de votre entreprise"}
          </h1>
          <p className="mt-2 text-base text-muted-foreground">
            {listOnly
              ? "Retrouvez le travail, les décisions et les livrables de chaque projet."
              : "Explorez les décisions, les projets et leurs liens."}
          </p>
        </div>
        <Button asChild className="min-h-11 shrink-0">
          <Link to="/projects/new">
            <Plus /> Nouveau projet
          </Link>
        </Button>
      </header>
      {!listOnly ? (
        <section
          className="grid gap-6 border-y py-7 lg:grid-cols-[minmax(0,1fr)_minmax(220px,0.65fr)]"
          aria-labelledby="company-context-heading"
        >
          <div>
            <div className="flex items-center gap-3">
              <span className="grid size-10 place-items-center rounded-lg bg-indigo-50 text-primary">
                <Network aria-hidden className="size-5" />
              </span>
              <h2
                id="company-context-heading"
                className="text-xl font-semibold"
              >
                {company.data?.workspace.name ?? "Votre espace partagé"}
              </h2>
            </div>
            <p className="mt-4 max-w-2xl text-sm leading-6 text-muted-foreground">
              {company.data?.workspace.description ||
                "Les décisions confirmées restent liées à leurs sources. Consultez le contexte commun et choisissez le projet dans lequel poursuivre votre travail."}
            </p>
            <div className="mt-5 flex flex-wrap gap-3">
              <Button asChild variant="outline">
                <Link to="/graph">
                  <Network /> Explorer le graphe
                </Link>
              </Button>
              <Button asChild variant="outline">
                <Link to="/agents">
                  <Bot /> Choisir un agent
                </Link>
              </Button>
            </div>
          </div>
          <div className="flex flex-col justify-center border-t pt-5 lg:border-l lg:border-t-0 lg:pl-7 lg:pt-0">
            {company.data && !company.data.setup_complete ? (
              <>
                <h3 className="font-medium">
                  Préparez votre espace entreprise
                </h3>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  Renseignez votre entreprise pour ouvrir son contexte général
                  et ses agents.
                </p>
                <Link
                  to="/company"
                  className="mt-4 flex items-center gap-2 text-sm font-medium text-primary"
                >
                  Configurer l’entreprise <ArrowRight className="size-4" />
                </Link>
              </>
            ) : (
              <>
                <h3 className="font-medium">
                  Gardez une trace de ce qui compte
                </h3>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  Examinez les points de cohérence avant de transmettre une
                  décision ou un livrable.
                </p>
                <Link
                  to="/insights"
                  className="mt-4 flex items-center gap-2 text-sm font-medium text-primary"
                >
                  Voir les points à vérifier <ArrowRight className="size-4" />
                </Link>
              </>
            )}
            <Link
              to="/settings/ai"
              className="mt-4 flex items-center gap-2 text-sm text-muted-foreground hover:text-primary"
            >
              <Settings2 className="size-4" /> Gérer mes connexions IA
            </Link>
          </div>
        </section>
      ) : null}
      <section aria-labelledby="projects-heading">
        <div className="mb-5 flex items-end justify-between gap-4">
          <h2 id="projects-heading" className="text-xl font-semibold">
            {listOnly ? "Tous les projets" : "Vos projets"}
          </h2>
          <span className="text-sm text-muted-foreground">
            {items.length} projet{items.length > 1 ? "s" : ""}
          </span>
        </div>
        {items.length === 0 ? (
          <EmptyState
            title="Votre premier projet commence ici"
            description="Donnez-lui un objectif, puis échangez avec un agent pour construire les premières décisions."
            action={
              <Button asChild>
                <Link to="/projects/new">Créer un projet</Link>
              </Button>
            }
          />
        ) : (
          <div className="divide-y rounded-lg border">
            {items.map((project) => (
              <article
                key={project.public_id}
                className="group flex flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between sm:px-6"
              >
                <div className="flex min-w-0 items-start gap-4">
                  <span className="mt-1 grid size-10 shrink-0 place-items-center rounded-lg bg-indigo-50 text-primary">
                    <FolderKanban aria-hidden className="size-5" />
                  </span>
                  <div className="min-w-0">
                    <h3 className="text-lg font-semibold">
                      <Link
                        to={`/projects/${project.public_id}`}
                        className="hover:text-primary"
                      >
                        {project.name}
                      </Link>
                    </h3>
                    <p className="mt-1 line-clamp-2 max-w-2xl text-sm leading-6 text-muted-foreground">
                      {project.objective ||
                        "Précisez l’objectif avec votre agent Produit."}
                    </p>
                    <p className="mt-2 text-xs text-muted-foreground">
                      Mis à jour {formatDate(project.updated_at)} · contexte v
                      {project.graph_version}
                    </p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-4 sm:pl-4">
                  <StatusPill
                    status={project.status}
                    label={project.status === "active" ? "Actif" : undefined}
                  />
                  <Button asChild variant="ghost">
                    <Link to={`/projects/${project.public_id}`}>
                      Ouvrir <ArrowRight />
                    </Link>
                  </Button>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
