import type { GraphEdge, GraphKind, GraphNode } from "@/api/company-types";

export const graphKindLabels: Record<GraphKind, string> = {
  scope: "Espace",
  session: "Conversation",
  task: "Tâche",
  agent: "Agent",
  knowledge: "Connaissance",
  artifact: "Livrable",
  external_reference: "Source externe",
  insight: "À vérifier",
};
export const graphRelationLabels: Record<string, string> = {
  contains: "Contient",
  belongs_to: "Appartient à",
  references: "Référence",
  depends_on: "Dépend de",
  informs: "Éclaire",
  supersedes: "Remplace",
  contradicts: "Contredit",
  derived_from: "Provient de",
  satisfies: "Répond à",
  evidenced_by: "Étayé par",
  implemented_by: "Mis en œuvre par",
  tracked_by: "Suivi dans",
};
export function nodeKey(node: Pick<GraphNode, "kind" | "id">) {
  return `${node.kind}:${node.id}`;
}
export function endpointKey(edge: GraphEdge, side: "source" | "target") {
  return `${edge[`${side}_kind`]}:${edge[`${side}_public_id`]}`;
}
export interface PositionedNode extends GraphNode {
  key: string;
  x: number;
  y: number;
}

/** Stable scope columns keep project boundaries legible without random force positions. */
export function layoutGraph(nodes: GraphNode[]) {
  const groups = new Map<string, GraphNode[]>();
  for (const node of nodes) {
    const group = groups.get(node.project_public_id) ?? [];
    group.push(node);
    groups.set(node.project_public_id, group);
  }
  const columns = [...groups.entries()].sort(([left, a], [right, b]) => {
    const company =
      Number(b.some((n) => n.scope_kind === "company")) -
      Number(a.some((n) => n.scope_kind === "company"));
    return company || left.localeCompare(right);
  });
  const positioned: PositionedNode[] = [];
  for (const [column, [, group]] of columns.entries()) {
    const sorted = [...group].sort(
      (a, b) =>
        Number(b.kind === "scope") - Number(a.kind === "scope") ||
        a.kind.localeCompare(b.kind) ||
        a.label.localeCompare(b.label) ||
        a.id.localeCompare(b.id),
    );
    sorted.forEach((node, row) =>
      positioned.push({
        ...node,
        key: nodeKey(node),
        x: 40 + column * 280,
        y: 48 + row * 96,
      }),
    );
  }
  return {
    nodes: positioned,
    width: Math.max(760, columns.length * 280 + 40),
    height: Math.max(520, ...positioned.map((n) => n.y + 112)),
  };
}

/** Only explicit edges whose endpoints are visible are drawable. */
export function visibleEdges(edges: GraphEdge[], nodes: GraphNode[]) {
  const keys = new Set(nodes.map(nodeKey));
  return edges.filter(
    (edge) =>
      keys.has(endpointKey(edge, "source")) &&
      keys.has(endpointKey(edge, "target")),
  );
}

export function sourceLink(node: GraphNode): string | null {
  if (!node.source_url) return null;
  try {
    const url = new URL(node.source_url);
    return url.protocol === "https:" && !url.username && !url.password
      ? url.href
      : null;
  } catch {
    return null;
  }
}

/** Only existing application routes with canonical IDs may be used as source links. */
export function internalSourceLink(
  node: Pick<GraphNode, "app_path">,
): string | null {
  const uuid = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
  const route = new RegExp(
    `^/(?:artifacts/${uuid}\\?version=${uuid}|projects/${uuid}/(?:(?:deliverables|insights|sessions)/${uuid}|sources/(?:knowledge|context_pack|task|artifact)/${uuid}|code\\?observation=${uuid}&file=${uuid}))$`,
  );
  return node.app_path && route.test(node.app_path) ? node.app_path : null;
}
