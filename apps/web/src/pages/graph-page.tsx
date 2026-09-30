import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import { Plus } from "lucide-react";
import { companyApi } from "@/api/company";
import { GraphExplorer } from "@/components/graph/graph-explorer";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export function GraphPage() {
  const [params, setParams] = useSearchParams();
  const projectId = params.get("project") || undefined;
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const graph = useQuery({
    queryKey: ["graph", projectId ?? "company"],
    queryFn: () => companyApi.graph(projectId),
  });
  const scopeNames = Object.fromEntries(
    company.data?.projects.map((project) => [
      project.public_id,
      project.name,
    ]) ?? [],
  );
  if (company.data?.company_scope)
    scopeNames[company.data.company_scope.project_public_id] =
      company.data.workspace.name;
  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow="Connaissances partagées"
        title="Le graphe de votre entreprise"
        description="Retrouvez ce qui relie les projets, les décisions et le travail de vos équipes."
        actions={
          <Button asChild variant="outline">
            <Link to="/projects/new">
              <Plus data-icon="inline-start" />
              Nouveau projet
            </Link>
          </Button>
        }
      />
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-sm text-muted-foreground">Explorer</span>
        <Select
          value={projectId ?? "company"}
          onValueChange={(value) =>
            setParams(value === "company" ? {} : { project: value })
          }
        >
          <SelectTrigger
            aria-label="Périmètre du graphe"
            className="w-full sm:w-72"
          >
            <SelectValue placeholder="Toute l’entreprise" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="company">Toute l’entreprise</SelectItem>
              {company.data?.projects.map((project) => (
                <SelectItem key={project.public_id} value={project.public_id}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
      {graph.isPending ? (
        <LoadingState label="Les connaissances et leurs liens se rassemblent…" />
      ) : graph.error ? (
        <ErrorState error={graph.error} retry={() => void graph.refetch()} />
      ) : (
        <GraphExplorer
          key={projectId ?? "company"}
          graph={graph.data}
          scopeNames={scopeNames}
          canEdit={
            company.data?.workspace.role === "owner" ||
            company.data?.workspace.role === "editor"
          }
        />
      )}
      {company.error && (
        <p role="status" className="text-sm text-muted-foreground">
          Les noms des projets n’ont pas pu être chargés.{" "}
          <button onClick={() => void company.refetch()} className="underline">
            Réessayer
          </button>
        </p>
      )}
    </div>
  );
}
