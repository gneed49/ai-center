import type { ContextPackSummary } from "@/api/types";

export function isContextPackCurrent(
  pack: ContextPackSummary | null | undefined,
  graphVersion: number,
) {
  return Boolean(
    pack &&
    pack.status === "current" &&
    pack.source_graph_version === graphVersion,
  );
}
