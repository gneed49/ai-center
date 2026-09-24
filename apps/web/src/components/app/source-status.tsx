import type { SourceStatus } from "@/api/types";
import { StatusPill } from "./status-pill";
export function SourceStatusBadge({
  status,
  plural = false,
}: {
  status?: SourceStatus | null;
  plural?: boolean;
}) {
  const value = status ?? "unknown";
  const label =
    value === "current"
      ? plural
        ? "Sources actuelles"
        : "Source actuelle"
      : value === "stale"
        ? plural
          ? "Sources dépassées"
          : "Source dépassée"
        : "Actualité non vérifiée";
  return <StatusPill status={value} label={label} />;
}
