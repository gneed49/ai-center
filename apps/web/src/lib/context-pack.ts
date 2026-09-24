import type { ContextPackSummary } from "@/api/types";

/** The server checks exact source versions and active scopes; graph counters are historical. */
export function isContextPackCurrent(
  pack: ContextPackSummary | null | undefined,
) {
  return pack?.status === "current";
}
