import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Archive, Download, RotateCcw } from "lucide-react";
import { Link } from "react-router";
import { companyControlsApi } from "@/api/company-controls";
import { createIdempotencyKey } from "@/api/client";
import type { ProjectSummary } from "@/api/types";
import {
  captureRequestContext,
  RequestContextChangedError,
} from "@/api/request-context";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";

export function CompanyDataControls({
  workspaceId,
  projects,
}: {
  workspaceId: string;
  projects: ProjectSummary[];
}) {
  const cache = useQueryClient();
  const archived = useQuery({
    queryKey: ["archived-projects"],
    queryFn: companyControlsApi.archivedProjects,
  });
  const previous = useRef<{ fingerprint: string; key: string } | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<Error | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const archive = useMutation({
    mutationFn: ({
      projectId,
      archived,
    }: {
      projectId: string;
      archived: boolean;
    }) => {
      const fingerprint = JSON.stringify([projectId, archived]);
      if (previous.current?.fingerprint !== fingerprint)
        previous.current = { fingerprint, key: createIdempotencyKey() };
      return companyControlsApi.archiveProject(
        projectId,
        archived,
        previous.current.key,
      );
    },
    onSuccess: async (project) => {
      previous.current = null;
      setNotice(
        project.status === "archived"
          ? "Projet archivé. Son historique reste disponible."
          : "Projet restauré. Préparez de nouveaux contextes de transmission avant de reprendre les agents techniques.",
      );
      await Promise.all([
        cache.invalidateQueries({ queryKey: ["company"] }),
        cache.invalidateQueries({ queryKey: ["projects"] }),
        cache.invalidateQueries({ queryKey: ["archived-projects"] }),
        cache.invalidateQueries({ queryKey: ["snapshot", project.public_id] }),
        cache.invalidateQueries({ queryKey: ["graph"] }),
      ]);
    },
  });
  async function exportData() {
    const context = captureRequestContext();
    setExporting(true);
    setExportError(null);
    setNotice(null);
    try {
      const text = await companyControlsApi.exportData();
      context.assertCurrent();
      const url = URL.createObjectURL(
        new Blob([text], { type: "application/json;charset=utf-8" }),
      );
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `ai-center-company-${workspaceId}-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.append(anchor);
      try {
        anchor.click();
      } finally {
        anchor.remove();
        setTimeout(() => URL.revokeObjectURL(url), 0);
      }
      setNotice(
        "Téléchargement de l’export complet demandé. Le fichier précise les données incluses et exclues.",
      );
    } catch (cause) {
      if (!(cause instanceof RequestContextChangedError))
        setExportError(
          cause instanceof Error
            ? cause
            : new Error("L’export n’a pas pu être préparé."),
        );
    } finally {
      setExporting(false);
    }
  }
  return (
    <section
      className="space-y-6 border-t pt-8"
      aria-labelledby="company-data-title"
    >
      <div>
        <h2 id="company-data-title" className="text-xl font-semibold">
          Conserver et organiser vos données
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">
          L’archive retire un projet du travail actif et préserve son
          historique. Aucun effacement définitif n’est effectué ici.
        </p>
      </div>
      <div className="flex flex-col justify-between gap-4 rounded-lg border p-5 sm:flex-row sm:items-center">
        <div>
          <h3 className="font-semibold">Export de l’entreprise</h3>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Téléchargez les données prises en charge et leur provenance. Les
            identifiants d’accès sensibles sont exclus ; le fichier détaille ces
            exclusions.
          </p>
        </div>
        <Button
          variant="outline"
          disabled={exporting}
          onClick={() => void exportData()}
        >
          <Download />
          {exporting ? "Préparation de l’export…" : "Exporter les données"}
        </Button>
      </div>
      {exportError ? (
        <ErrorState title="L’export n’a pas abouti" error={exportError} />
      ) : null}
      {notice ? (
        <p role="status" className="text-sm text-emerald-800">
          {notice}
        </p>
      ) : null}
      <details className="rounded-lg border p-5">
        <summary className="font-semibold">
          Projets actifs · {projects.length}
        </summary>
        {projects.length ? (
          <ul className="mt-4 divide-y">
            {projects.map((project) => (
              <li
                key={project.public_id}
                className="flex flex-wrap items-center justify-between gap-3 py-4"
              >
                <Link
                  to={`/projects/${project.public_id}`}
                  className="break-words text-sm font-medium text-primary"
                >
                  {project.name}
                </Link>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={archive.isPending}
                  onClick={() =>
                    archive.mutate({
                      projectId: project.public_id,
                      archived: true,
                    })
                  }
                >
                  <Archive />
                  Archiver ce projet
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="mt-4 text-sm text-muted-foreground">
            Aucun projet actif.
          </p>
        )}
      </details>
      <div className="rounded-lg border p-5">
        <h3 className="font-semibold">Projets archivés</h3>
        {archived.isPending ? (
          <LoadingState label="Chargement des archives…" />
        ) : archived.isError ? (
          <ErrorState
            error={archived.error}
            retry={() => void archived.refetch()}
          />
        ) : archived.data.length ? (
          <ul className="mt-4 divide-y">
            {archived.data.map((project) => (
              <li
                key={project.public_id}
                className="flex flex-wrap items-center justify-between gap-3 py-4"
              >
                <Link
                  to={`/projects/${project.public_id}/history`}
                  className="break-words text-sm font-medium text-primary"
                >
                  {project.name} · consulter l’historique
                </Link>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={archive.isPending}
                  onClick={() =>
                    archive.mutate({
                      projectId: project.public_id,
                      archived: false,
                    })
                  }
                >
                  <RotateCcw />
                  Restaurer le projet
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">
            Aucun projet archivé.
          </p>
        )}
        <p className="mt-4 text-xs leading-5 text-muted-foreground">
          La restauration ne réactive pas les anciens packs de contexte : ils
          restent dépassés et doivent être préparés de nouveau.
        </p>
      </div>
      {archive.error ? (
        <ErrorState
          title="Le changement d’archive n’a pas pu être confirmé"
          error={archive.error}
          retry={
            archive.variables
              ? () => archive.mutate(archive.variables!)
              : undefined
          }
        />
      ) : null}
    </section>
  );
}
