import type { SourceObservation } from "@/api/tool-source-types";
export function observedMetadata(
  value: Record<string, unknown> | null | undefined,
): Partial<SourceObservation> | undefined {
  if (value?.trust !== "observed_external") return undefined;
  return {
    trust: "observed_external",
    mandatory: false,
    provider:
      value.provider === "notion" || value.provider === "linear"
        ? value.provider
        : undefined,
    version: typeof value.version === "number" ? value.version : undefined,
    observed_at:
      typeof value.observed_at === "string" ? value.observed_at : undefined,
    coverage:
      value.coverage === "complete" ||
      value.coverage === "partial" ||
      value.coverage === "none"
        ? value.coverage
        : undefined,
    omission_reasons: Array.isArray(value.omission_reasons)
      ? value.omission_reasons.filter((v): v is string => typeof v === "string")
      : undefined,
  };
}
