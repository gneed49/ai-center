import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";
import { companyApi } from "@/api/company";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { internalSourceLink } from "@/lib/graph-layout";
import { formatDate } from "@/lib/format";

const labels: Record<string, string> = {
  knowledge: "Connaissance",
  context_pack: "Contexte transmis",
  task: "Tâche",
  artifact: "Livrable référencé",
};

export function GraphSourcePage() {
  const { projectId = "", kind = "", sourceId = "" } = useParams();
  const source = useQuery({
    queryKey: ["graph-source", projectId, kind, sourceId],
    queryFn: async () => {
      const result = await companyApi.source(projectId, kind, sourceId);
      if (
        result.public_id !== sourceId ||
        result.project_public_id !== projectId ||
        result.kind !== kind
      )
        throw new Error(
          "La source reçue ne correspond pas à la version demandée.",
        );
      return result;
    },
  });
  if (source.isPending) return <LoadingState />;
  if (source.error)
    return <ErrorState error={source.error} retry={() => source.refetch()} />;
  const record = source.data;
  const historical = ["stale", "superseded"].includes(record.status);
  return (
    <div className="space-y-6">
      <nav
        aria-label="Contexte de la source"
        className="flex flex-wrap gap-3 text-sm text-primary"
      >
        <Link
          to={
            record.scope_kind === "company"
              ? "/company"
              : `/projects/${projectId}`
          }
        >
          {record.project_name}
        </Link>
        <Link to={`/graph?project=${projectId}`}>Graphe du projet</Link>
      </nav>
      <PageHeader
        eyebrow={labels[record.kind]}
        title={record.title}
        description="Consultez la source enregistrée et retrouvez le contexte dont elle provient."
        actions={
          <Button variant="outline" asChild>
            <Link to={`/graph?project=${projectId}`}>Revenir au graphe</Link>
          </Button>
        }
      />
      <div className="flex flex-wrap items-center gap-3 text-sm">
        <StatusPill status={record.status} />
        {record.version_number !== null && (
          <span>Version {record.version_number}</span>
        )}
        <time dateTime={record.recorded_at}>
          {formatDate(record.recorded_at, true)}
        </time>
      </div>
      {historical && (
        <p
          role="status"
          className="rounded border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900"
        >
          Vous consultez une version historique. Son contenu est conservé tel
          qu’il a été enregistré.
        </p>
      )}
      <section
        aria-label="Contenu de la source"
        className="space-y-5 rounded border border-border bg-card p-5 sm:p-7"
      >
        <SourceContent content={record.content} />
      </section>
      <section aria-label="Provenance" className="space-y-3">
        <h2 className="text-lg font-semibold">Sources liées</h2>
        {record.links.length ? (
          <ul className="space-y-2">
            {record.links.map((link) => {
              const path = internalSourceLink(link);
              return (
                <li key={`${link.app_path}:${link.label}`}>
                  {path ? (
                    <Link
                      className="text-primary underline underline-offset-4"
                      to={path}
                    >
                      {link.label}
                    </Link>
                  ) : (
                    <span>{link.label} — lien indisponible</span>
                  )}
                </li>
              );
            })}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">
            Aucune source navigable n’est enregistrée pour cet élément.
          </p>
        )}
      </section>
      <details className="rounded border border-border p-4">
        <summary className="cursor-pointer text-sm font-medium">
          Données enregistrées de cette source
        </summary>
        <pre className="mt-3 max-h-96 overflow-auto whitespace-pre-wrap break-words text-xs">
          {JSON.stringify(record.content, null, 2)}
        </pre>
      </details>
    </div>
  );
}

function SourceContent({ content }: { content: Record<string, unknown> }) {
  const texts = ["objective", "project_summary", "statement", "rationale"]
    .map((key) => content[key])
    .filter(
      (value): value is string => typeof value === "string" && value.length > 0,
    );
  const knowledge = Array.isArray(content.knowledge) ? content.knowledge : [];
  return (
    <>
      {texts.map((text, index) => (
        <p
          key={index}
          className="whitespace-pre-wrap break-words leading-relaxed"
        >
          {text}
        </p>
      ))}
      {knowledge.map((item: unknown, index) => {
        if (
          !item ||
          typeof item !== "object" ||
          !("statement" in item) ||
          typeof item.statement !== "string"
        )
          return null;
        return (
          <article key={index} className="border-t border-border pt-4">
            {"title" in item && typeof item.title === "string" && (
              <h2 className="font-semibold">{item.title}</h2>
            )}
            <p className="mt-2 whitespace-pre-wrap break-words text-sm leading-relaxed">
              {item.statement}
            </p>
          </article>
        );
      })}
      {!texts.length && !knowledge.length && (
        <p className="text-sm text-muted-foreground">
          Le contenu structuré est disponible dans les données enregistrées
          ci-dessous.
        </p>
      )}
    </>
  );
}
