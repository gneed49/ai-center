import { useMemo, useState } from "react";
import { Link } from "react-router";
import {
  ArrowUpRight,
  MessageSquare,
  ListTodo,
  Bot,
  Building2,
  FileText,
  Folder,
  GitBranch,
  List,
  Network,
  Search,
  ShieldAlert,
  ZoomIn,
  ZoomOut,
} from "lucide-react";

import type { GraphKind, GraphNode, GraphView } from "@/api/company-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { EmptyState } from "@/components/app/page";
import {
  endpointKey,
  graphKindLabels,
  graphRelationLabels,
  layoutGraph,
  nodeKey,
  sourceLink,
  internalSourceLink,
  visibleEdges,
} from "@/lib/graph-layout";
import { cn } from "@/lib/utils";
import { GraphRelationForm } from "./graph-relation-form";

const icons = {
  scope: Folder,
  session: MessageSquare,
  task: ListTodo,
  agent: Bot,
  knowledge: FileText,
  artifact: FileText,
  external_reference: ArrowUpRight,
  insight: ShieldAlert,
};
const kindStyles: Record<GraphKind, string> = {
  session: "border-violet-200 bg-violet-50 text-violet-900",
  task: "border-sky-200 bg-sky-50 text-sky-900",
  scope: "border-indigo-300 bg-indigo-50 text-indigo-900",
  agent: "border-slate-200 bg-white text-slate-700",
  knowledge: "border-teal-200 bg-teal-50 text-teal-900",
  artifact: "border-indigo-200 bg-white text-indigo-900",
  external_reference: "border-slate-200 bg-white text-slate-700",
  insight: "border-amber-200 bg-amber-50 text-amber-900",
};

export function GraphExplorer({
  graph,
  scopeNames,
  canEdit = false,
}: {
  graph: GraphView;
  scopeNames: Record<string, string>;
  canEdit?: boolean;
}) {
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("all");
  const [mode, setMode] = useState<"graph" | "list">("graph");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const nodes = useMemo(
    () =>
      graph.nodes.filter(
        (node) =>
          (kind === "all" || node.kind === kind) &&
          node.label
            .toLocaleLowerCase("fr")
            .includes(search.trim().toLocaleLowerCase("fr")),
      ),
    [graph.nodes, kind, search],
  );
  const layout = useMemo(() => layoutGraph(nodes), [nodes]);
  const edges = useMemo(
    () => visibleEdges(graph.edges, nodes),
    [graph.edges, nodes],
  );
  const positions = useMemo(
    () => new Map(layout.nodes.map((node) => [node.key, node])),
    [layout],
  );
  const selected = nodes.find((node) => nodeKey(node) === selectedKey);
  const relations = selected
    ? graph.edges.filter(
        (edge) =>
          endpointKey(edge, "source") === selectedKey ||
          endpointKey(edge, "target") === selectedKey,
      )
    : [];
  const allNodes = new Map(graph.nodes.map((node) => [nodeKey(node), node]));

  function selectNode(node: GraphNode) {
    setSelectedKey(nodeKey(node));
  }

  return (
    <section
      aria-label="Explorer le graphe"
      className="overflow-hidden rounded-xl border border-border bg-background"
    >
      <div className="flex flex-wrap items-center gap-3 border-b border-border p-4">
        <div className="relative min-w-48 flex-1">
          <Search
            aria-hidden
            className="pointer-events-none absolute left-3 top-2.5 size-4 text-muted-foreground"
          />
          <Input
            aria-label="Rechercher dans le graphe"
            placeholder="Rechercher une règle, un livrable, un agent…"
            className="pl-9"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </div>
        <Select value={kind} onValueChange={setKind}>
          <SelectTrigger className="w-44" aria-label="Type d’élément">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="all">Tous les éléments</SelectItem>
              {Object.entries(graphKindLabels).map(([value, label]) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        <div
          className="flex gap-1"
          role="group"
          aria-label="Présentation du graphe"
        >
          <Button
            variant={mode === "graph" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setMode("graph")}
            aria-pressed={mode === "graph"}
            className="aria-pressed:bg-primary/10 aria-pressed:text-primary"
          >
            <Network data-icon="inline-start" />
            Graphe
          </Button>
          <Button
            variant={mode === "list" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setMode("list")}
            aria-pressed={mode === "list"}
            className="aria-pressed:bg-primary/10 aria-pressed:text-primary"
          >
            <List data-icon="inline-start" />
            Liste
          </Button>
        </div>
      </div>
      {graph.truncated && (
        <p
          role="status"
          className="border-b border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900"
        >
          Cette vue est limitée à {graph.limits.max_nodes} éléments et{" "}
          {graph.limits.max_edges} liens. Ouvrez un projet pour explorer son
          contexte plus en détail.
        </p>
      )}
      <div className="grid min-h-[34rem] xl:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="min-w-0">
          {!nodes.length ? (
            <div className="p-6">
              <EmptyState
                title={
                  graph.nodes.length
                    ? "Aucun élément ne correspond"
                    : "Le contexte prend forme ici"
                }
                description={
                  graph.nodes.length
                    ? "Essayez un autre mot ou un autre type d’élément."
                    : "Créez un projet et commencez une conversation pour construire ses connaissances et leurs liens."
                }
                action={
                  graph.nodes.length ? (
                    <Button
                      variant="outline"
                      onClick={() => {
                        setSearch("");
                        setKind("all");
                      }}
                    >
                      Effacer les filtres
                    </Button>
                  ) : (
                    <Button asChild>
                      <Link to="/projects/new">Créer un projet</Link>
                    </Button>
                  )
                }
              />
            </div>
          ) : mode === "list" ? (
            <ul
              className="divide-y divide-border"
              aria-label="Éléments du graphe"
            >
              {nodes.map((node) => (
                <li key={nodeKey(node)}>
                  <button
                    className={cn(
                      "flex w-full items-center gap-3 px-5 py-4 text-left hover:bg-muted/50 focus-visible:outline-2 focus-visible:outline-primary",
                      selectedKey === nodeKey(node) && "bg-primary/5",
                    )}
                    onClick={() => selectNode(node)}
                    aria-pressed={selectedKey === nodeKey(node)}
                  >
                    <NodeIcon node={node} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">
                        {node.label}
                      </span>
                      <span className="block text-xs text-muted-foreground">
                        {graphKindLabels[node.kind]} ·{" "}
                        {node.version_number
                          ? `v${node.version_number} · `
                          : ""}
                        {node.status === "superseded" ? "Historique · " : ""}
                        {scopeNames[node.project_public_id] ?? "Projet"}
                      </span>
                    </span>
                    <ArrowUpRight
                      aria-hidden
                      className="size-4 text-muted-foreground"
                    />
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <>
              <div className="flex items-center justify-between border-b border-border px-4 py-2">
                <p className="text-xs text-muted-foreground" aria-live="polite">
                  {nodes.length} éléments · {edges.length} liens visibles
                </p>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="Réduire le graphe"
                    disabled={zoom <= 0.5}
                    onClick={() => setZoom(Math.max(0.5, zoom - 0.25))}
                  >
                    <ZoomOut />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label="Rétablir le zoom"
                    onClick={() => setZoom(1)}
                  >
                    {Math.round(zoom * 100)} %
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="Agrandir le graphe"
                    disabled={zoom >= 1.5}
                    onClick={() => setZoom(Math.min(1.5, zoom + 0.25))}
                  >
                    <ZoomIn />
                  </Button>
                </div>
              </div>
              <div
                className="max-h-[42rem] overflow-auto bg-slate-50/60"
                tabIndex={0}
                role="region"
                aria-label="Carte des relations ; les éléments sont accessibles au clavier"
              >
                <div
                  className="relative"
                  style={{
                    width: layout.width * zoom,
                    height: layout.height * zoom,
                  }}
                >
                  <div
                    className="absolute inset-0 origin-top-left"
                    style={{
                      width: layout.width,
                      height: layout.height,
                      transform: `scale(${zoom})`,
                    }}
                  >
                    <svg
                      aria-hidden="true"
                      width={layout.width}
                      height={layout.height}
                      className="pointer-events-none absolute inset-0 overflow-visible"
                    >
                      <defs>
                        <marker
                          id="relation-arrow"
                          viewBox="0 0 10 10"
                          refX="9"
                          refY="5"
                          markerWidth="5"
                          markerHeight="5"
                          orient="auto"
                        >
                          <path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" />
                        </marker>
                      </defs>
                      {edges.map((edge) => {
                        const from = positions.get(
                          endpointKey(edge, "source"),
                        )!;
                        const to = positions.get(endpointKey(edge, "target"))!;
                        const sameColumn = from.x === to.x;
                        const startX = sameColumn
                          ? from.x + 236
                          : from.x + (from.x < to.x ? 236 : 0);
                        const endX = sameColumn
                          ? to.x + 236
                          : to.x + (from.x < to.x ? 0 : 236);
                        const control = sameColumn
                          ? startX +
                            22 +
                            Math.min(18, Math.abs(from.y - to.y) / 20)
                          : (startX + endX) / 2;
                        const highlighted =
                          endpointKey(edge, "source") === selectedKey ||
                          endpointKey(edge, "target") === selectedKey;
                        return (
                          <path
                            key={edge.id}
                            d={`M ${startX} ${from.y + 32} C ${control} ${from.y + 32}, ${control} ${to.y + 32}, ${endX} ${to.y + 32}`}
                            fill="none"
                            stroke={highlighted ? "#4338ca" : "#cbd5e1"}
                            strokeWidth={highlighted ? 2 : 1.25}
                            strokeDasharray={
                              edge.provenance.origin === "scope_membership"
                                ? "4 4"
                                : undefined
                            }
                            markerEnd="url(#relation-arrow)"
                            className={
                              highlighted ? "text-indigo-700" : "text-slate-400"
                            }
                          />
                        );
                      })}
                    </svg>
                    {layout.nodes.map((node) => (
                      <button
                        key={node.key}
                        data-node-key={node.key}
                        type="button"
                        style={{ left: node.x, top: node.y }}
                        className={cn(
                          "absolute flex h-16 w-[236px] items-center gap-3 rounded-lg border p-3 text-left shadow-sm transition-shadow hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary",
                          kindStyles[node.kind],
                          selectedKey === node.key &&
                            "ring-2 ring-primary ring-offset-2",
                        )}
                        onClick={() => selectNode(node)}
                        aria-pressed={selectedKey === node.key}
                        aria-label={`${node.label}${node.version_number ? `, version ${node.version_number}` : ""}, ${graphKindLabels[node.kind]}, ${scopeNames[node.project_public_id] ?? "Projet"}`}
                      >
                        <NodeIcon node={node} />
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium">
                            {node.label}
                          </span>
                          <span className="block truncate text-[11px] opacity-75">
                            {node.kind === "scope"
                              ? node.scope_kind === "company"
                                ? "Contexte de l’entreprise"
                                : "Projet partagé"
                              : graphKindLabels[node.kind]}
                            {node.version_number
                              ? ` · v${node.version_number}`
                              : ""}
                            {node.status === "superseded"
                              ? " · Historique"
                              : ""}
                          </span>
                        </span>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </>
          )}
        </div>
        <aside
          className="border-t border-border p-5 xl:border-l xl:border-t-0"
          aria-label="Détails de l’élément"
          aria-live="polite"
        >
          {selected ? (
            <>
              <div className="mb-4 flex items-center gap-2 text-xs text-muted-foreground">
                <NodeIcon node={selected} />
                {graphKindLabels[selected.kind]}
              </div>
              <h2 className="break-words text-lg font-semibold leading-6">
                {selected.label}
              </h2>
              {selected.version_number && (
                <p className="mt-2 text-sm text-muted-foreground">
                  Version {selected.version_number}
                  {selected.status === "superseded" ? " · Historique" : ""}
                  {selected.status === "validated" ? " · Validée" : ""}
                </p>
              )}
              {selected.status === "stale" && (
                <p className="mt-3 rounded-md bg-amber-50 p-3 text-sm text-amber-900">
                  À actualiser : des sources de cet élément ont changé. Vérifiez
                  leur version avant de poursuivre.
                </p>
              )}
              {selected.status === "draft" && (
                <Badge variant="outline" className="mt-3">
                  Brouillon à relire
                </Badge>
              )}
              <p className="mt-2 text-sm text-muted-foreground">
                {scopeNames[selected.project_public_id] ?? "Projet partagé"}
              </p>
              {selected.version_public_id && (
                <details className="mt-4 text-xs text-muted-foreground">
                  <summary className="cursor-pointer">
                    Version de la source
                  </summary>
                  <code className="mt-2 block break-all">
                    {selected.version_public_id}
                  </code>
                </details>
              )}
              <div className="my-5 flex flex-wrap gap-2">
                {selected.scope_kind === "project" && (
                  <Button asChild size="sm" variant="outline">
                    <Link to={`/projects/${selected.project_public_id}`}>
                      Ouvrir le projet
                      <ArrowUpRight data-icon="inline-end" />
                    </Link>
                  </Button>
                )}
                {internalSourceLink(selected) ? (
                  <Button asChild size="sm" variant="outline">
                    <Link to={internalSourceLink(selected)!}>
                      Ouvrir la source
                      <ArrowUpRight data-icon="inline-end" />
                    </Link>
                  </Button>
                ) : sourceLink(selected) ? (
                  <Button asChild size="sm" variant="outline">
                    <a
                      href={sourceLink(selected)!}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      Ouvrir la source
                      <ArrowUpRight data-icon="inline-end" />
                    </a>
                  </Button>
                ) : null}
              </div>
              <h3 className="border-t border-border pt-5 text-sm font-medium">
                Ce qui relie cet élément
              </h3>
              {relations.length ? (
                <ul className="mt-3 space-y-3">
                  {relations.map((edge) => {
                    const outgoing =
                      endpointKey(edge, "source") === selectedKey;
                    const other = allNodes.get(
                      endpointKey(edge, outgoing ? "target" : "source"),
                    );
                    return (
                      <li
                        key={edge.id}
                        className="rounded-md border border-border p-3"
                      >
                        <p className="text-xs text-muted-foreground">
                          {outgoing ? "Vers" : "Depuis"} ·{" "}
                          {graphRelationLabels[edge.edge_type] ??
                            edge.edge_type}
                        </p>
                        {other ? (
                          <button
                            className="mt-1 text-left text-sm font-medium text-primary underline-offset-4 hover:underline"
                            onClick={() => {
                              setSearch("");
                              setKind("all");
                              selectNode(other);
                            }}
                          >
                            {other.label}
                          </button>
                        ) : (
                          <p className="mt-1 text-sm">
                            Source hors de cette vue
                          </p>
                        )}
                        {edge.provenance.origin === "scope_membership" && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            Appartenance au projet
                          </p>
                        )}
                        {edge.provenance.origin === "human" && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            Lien confirmé par l’équipe
                          </p>
                        )}
                        {edge.provenance.origin === "source_record" && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            Source enregistrée avec le livrable
                          </p>
                        )}
                      </li>
                    );
                  })}
                </ul>
              ) : (
                <p className="mt-3 text-sm text-muted-foreground">
                  Aucun lien affiché pour cet élément.
                </p>
              )}
              {canEdit && (
                <GraphRelationForm
                  key={nodeKey(selected)}
                  source={selected}
                  nodes={graph.nodes}
                />
              )}
            </>
          ) : (
            <div className="py-8">
              <GitBranch className="mb-4 size-7 text-primary" aria-hidden />
              <h2 className="text-base font-semibold">Le contexte, relié</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                Sélectionnez un élément pour comprendre ses liens, retrouver sa
                source et poursuivre le travail.
              </p>
              <div className="mt-6 flex flex-wrap gap-2">
                {Object.entries(graphKindLabels).map(([value, label]) => (
                  <Badge key={value} variant="outline">
                    {label}
                  </Badge>
                ))}
              </div>
              <p className="mt-5 text-xs leading-5 text-muted-foreground">
                Les liens affichés viennent des données du projet. Une proximité
                dans le dessin ne signifie pas une relation.
              </p>
            </div>
          )}
        </aside>
      </div>
    </section>
  );
}

function NodeIcon({ node }: { node: GraphNode }) {
  const Icon =
    node.kind === "scope" && node.scope_kind === "company"
      ? Building2
      : icons[node.kind];
  return <Icon aria-hidden className="size-5 shrink-0" />;
}
