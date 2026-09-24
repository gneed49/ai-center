import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import { companyApi } from "@/api/company";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { humanize } from "@/lib/format";

export function KnowledgeLibraryPage() {
  const [params, setParams] = useSearchParams();
  const project = params.get("project") || undefined;
  const q = params.get("q") ?? "";
  const history = params.get("history") === "true";
  const parsed = Number(params.get("offset") ?? "0");
  const offset =
    Number.isSafeInteger(parsed) && parsed >= 0 && parsed <= 1_000_000
      ? parsed
      : 0;
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const library = useQuery({
    queryKey: ["knowledge-library", project, q, history, offset],
    queryFn: () =>
      companyApi.knowledge({ project_id: project, q, history, offset }),
  });
  function update(values: Record<string, string>) {
    const next = new URLSearchParams(params);
    for (const [key, value] of Object.entries(values))
      if (value) next.set(key, value);
      else next.delete(key);
    setParams(next);
  }
  const projects = company.data?.projects ?? [];
  const companyScope = company.data?.company_scope?.project_public_id;
  const unknownProject =
    project &&
    project !== companyScope &&
    !projects.some((item) => item.public_id === project);
  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow="Mémoire partagée"
        title="Connaissances de l’entreprise"
        description="Retrouvez les règles, décisions et exigences de vos projets, avec leurs versions et leurs sources. Les projets archivés restent consultables."
        actions={
          <Button variant="outline" asChild>
            <Link to={project ? `/graph?project=${project}` : "/graph"}>
              Explorer le graphe
            </Link>
          </Button>
        }
      />
      <form
        className="flex flex-wrap items-end gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          update({
            q: String(new FormData(event.currentTarget).get("q") ?? "").trim(),
            offset: "",
          });
        }}
      >
        <label className="min-w-0 flex-1 space-y-2 text-sm font-medium">
          Rechercher une connaissance
          <Input
            key={q}
            name="q"
            defaultValue={q}
            maxLength={200}
            placeholder="Un sujet, une règle ou une décision…"
            className="mt-2"
          />
        </label>
        <Button type="submit">Rechercher</Button>
      </form>
      <div className="flex flex-wrap items-center gap-5">
        <label className="flex flex-wrap items-center gap-2 text-sm">
          Périmètre des connaissances
          <select
            className="max-w-full rounded-md border bg-background p-2"
            value={project ?? ""}
            onChange={(event) =>
              update({ project: event.target.value, offset: "" })
            }
          >
            <option value="">Toute l’entreprise</option>
            {companyScope && (
              <option value={companyScope}>
                Contexte général de l’entreprise
              </option>
            )}
            {projects.map((item) => (
              <option key={item.public_id} value={item.public_id}>
                {item.name}
              </option>
            ))}
            {unknownProject && (
              <option value={project}>
                {library.data?.items[0]?.project_name ?? "Projet sélectionné"}
              </option>
            )}
          </select>
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={history}
            onChange={(event) =>
              update({
                history: event.target.checked ? "true" : "",
                offset: "",
              })
            }
          />
          Inclure les versions historiques
        </label>
      </div>
      {company.error && (
        <p role="alert" className="text-sm text-destructive">
          La liste des projets est indisponible.{" "}
          <button className="underline" onClick={() => void company.refetch()}>
            Réessayer
          </button>
        </p>
      )}
      {library.isPending ? (
        <LoadingState />
      ) : library.error ? (
        <ErrorState error={library.error} retry={() => library.refetch()} />
      ) : (
        <>
          <p role="status" className="text-sm text-muted-foreground">
            {library.data.total} résultat(s) ·{" "}
            {library.data.items.length
              ? `${offset + 1}–${offset + library.data.items.length}`
              : "0 affiché"}{" "}
            · {history ? "Toutes les versions" : "Versions courantes"}
          </p>
          {library.data.items.length ? (
            <ul className="divide-y rounded-lg border bg-card">
              {library.data.items.map((item) => (
                <li key={item.public_id} className="space-y-2 p-5">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <Link
                      className="font-semibold text-primary underline-offset-4 hover:underline"
                      to={`/projects/${item.project_public_id}/sources/knowledge/${item.public_id}`}
                    >
                      {item.title}
                    </Link>
                    <span className="text-xs text-muted-foreground">
                      Version {item.version_number}
                      {item.status === "superseded" ? " · Historique" : ""}
                    </span>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {item.project_name} · {humanize(item.entry_type)}
                    {item.project_status === "archived"
                      ? " · Projet archivé"
                      : ""}
                  </p>
                  <p className="break-words text-sm leading-relaxed text-muted-foreground">
                    {item.excerpt}
                  </p>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState
              title="Aucune connaissance pour cette recherche"
              description="Essayez un autre terme ou incluez les versions historiques."
            />
          )}
          <nav aria-label="Pages de connaissances" className="flex gap-3">
            <Button
              variant="outline"
              disabled={offset === 0}
              onClick={() =>
                update({ offset: String(Math.max(0, offset - 25)) })
              }
            >
              Page précédente
            </Button>
            <Button
              variant="outline"
              disabled={
                offset + library.data.items.length >= library.data.total
              }
              onClick={() => update({ offset: String(offset + 25) })}
            >
              Page suivante
            </Button>
          </nav>
        </>
      )}
    </div>
  );
}
