import type { CodeFileStatus } from "@/api/code-observations";
export const codeFileLabels: Record<CodeFileStatus, string> = {
  code_read: "Texte lu",
  missing: "Fichier absent",
  inaccessible: "Accès refusé",
  too_large: "Fichier trop volumineux",
  binary: "Fichier binaire",
  unsupported: "Format non pris en charge",
  unavailable: "Lecture indisponible",
};
export function codeReadValidation(
  repository: string,
  commit: string,
  paths: string[],
): string | null {
  if (
    !/^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+$/.test(repository) ||
    repository
      .split("/")
      .some((part) => part.length > 100 || [".", ".."].includes(part))
  )
    return "Indiquez le dépôt sous la forme organisation/dépôt.";
  if (!/^[a-f0-9]{40}$/i.test(commit))
    return "Indiquez l’identifiant complet du commit (40 caractères), pas un nom de branche.";
  if (
    paths.length < 1 ||
    paths.length > 10 ||
    new Set(paths).size !== paths.length
  )
    return "Choisissez entre un et dix fichiers différents, un chemin par ligne.";
  if (
    paths.some(
      (path) =>
        path.length > 512 ||
        path.includes("\\") ||
        /\p{Cc}/u.test(path) ||
        path.split("/").length > 8 ||
        path.split("/").some((part) => ["", ".", ".."].includes(part)),
    )
  )
    return "Les chemins doivent être relatifs au dépôt, sans remontée de dossier et avec huit niveaux au maximum.";
  return null;
}
export function githubCodeUrl(
  repository: string,
  commit: string,
  path: string,
  lineStart?: number,
  lineEnd?: number,
) {
  if (codeReadValidation(repository, commit, [path])) return null;
  const url = new URL(
    `https://github.com/${repository.split("/").map(encodeURIComponent).join("/")}/blob/${commit}/${path.split("/").map(encodeURIComponent).join("/")}`,
  );
  if (lineStart && Number.isInteger(lineStart) && lineStart > 0)
    url.hash = `L${lineStart}${lineEnd && Number.isInteger(lineEnd) && lineEnd >= lineStart ? `-L${lineEnd}` : ""}`;
  return url.toString();
}

export function codeObservationLabel(
  status: CodeFileStatus,
  reason: string | null,
) {
  return reason === "sensitive_content_excluded"
    ? "Contenu sensible exclu"
    : codeFileLabels[status];
}
