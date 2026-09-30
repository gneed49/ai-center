import type {
  PublicationStatus,
  WorkToolProvider,
} from "@/api/work-tool-types";
export const publicationLabels: Record<PublicationStatus, string> = {
  queued: "En attente",
  processing: "En cours",
  succeeded: "Publication confirmée",
  failed: "Publication non aboutie",
  needs_review: "Résultat à vérifier",
  conflict: "Modification externe détectée",
  unavailable: "Source externe indisponible",
  cancelled: "Annulée",
};
export function safeWorkToolUrl(
  provider: WorkToolProvider,
  value: string | null,
) {
  if (!value) return null;
  try {
    const url = new URL(value);
    const hosts = {
      notion: ["www.notion.so", "notion.so"],
      linear: ["linear.app"],
      github: ["github.com"],
    };
    return url.protocol === "https:" &&
      !url.username &&
      !url.password &&
      !url.port &&
      hosts[provider].includes(url.hostname)
      ? url.toString()
      : null;
  } catch {
    return null;
  }
}
export function validExternalObjectId(
  provider: WorkToolProvider,
  value: string,
) {
  return provider === "github"
    ? /^[1-9][0-9]{0,14}$/.test(value)
    : /^[a-f0-9]{8}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{12}$/i.test(
        value,
      );
}
