import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import { companyApi } from "@/api/company";
import { DestinationSettings } from "@/components/artifacts/destination-settings";
import { ErrorState, LoadingState, NotFoundState } from "@/components/app/page";
import { Button } from "@/components/ui/button";

export function ArtifactDestinationsPage() {
  const [params, setParams] = useSearchParams();
  const projectId = params.get("project") ?? undefined;
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  if (company.isPending)
    return <LoadingState label="Chargement des réglages…" />;
  if (company.isError)
    return (
      <ErrorState error={company.error} retry={() => void company.refetch()} />
    );
  if (
    projectId &&
    !company.data.projects.some((project) => project.public_id === projectId)
  )
    return <NotFoundState title="Projet inaccessible" />;
  return (
    <div className="space-y-7">
      <header>
        <h1 className="text-3xl font-semibold tracking-tight">
          Où conserver vos livrables
        </h1>
        <p className="mt-2 text-muted-foreground">
          Définissez les destinations proposées à l’équipe.
        </p>
      </header>
      <div className="flex flex-wrap items-end justify-between gap-4 border-b pb-6">
        <div>
          <label
            htmlFor="destination-scope"
            className="mb-2 block text-sm font-medium"
          >
            Réglages pour
          </label>
          <select
            id="destination-scope"
            className="min-h-11 rounded-md border bg-white px-3 text-sm"
            value={projectId ?? ""}
            onChange={(event) =>
              setParams(
                event.target.value ? { project: event.target.value } : {},
              )
            }
          >
            <option value="">Entreprise · {company.data.workspace.name}</option>
            {company.data.projects.map((project) => (
              <option key={project.public_id} value={project.public_id}>
                Projet · {project.name}
              </option>
            ))}
          </select>
        </div>
        <Button asChild variant="outline">
          <Link
            to={projectId ? `/projects/${projectId}/artifacts` : "/artifacts"}
          >
            Voir les livrables
          </Link>
        </Button>
      </div>
      <DestinationSettings
        key={projectId ?? "company"}
        projectId={projectId}
        role={company.data.workspace.role}
      />
    </div>
  );
}
