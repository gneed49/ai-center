export const sourceProviderLabels = { linear: "Linear", notion: "Notion" };
export const coverageLabels = {
  complete: "Texte lu dans le périmètre annoncé",
  partial: "Lecture partielle",
  none: "Aucun contenu disponible",
};
export const omissionLabels: Record<string, string> = {
  comments_not_read: "Commentaires non lus",
  attachments_not_read: "Pièces jointes non lues",
  related_objects_not_read: "Objets liés non lus",
  properties_not_read: "Propriétés non lues",
  embedded_content_not_read: "Contenus intégrés non lus",
  transcripts_not_read: "Transcriptions non lues",
  unknown_blocks: "Certains blocs n’ont pas pu être lus",
  provider_truncated: "Texte tronqué par l’outil",
  local_text_limit: "Limite de texte atteinte",
  remote_date_unavailable: "Date de modification distante indisponible",
};
export const freshnessLabels: Record<string, string> = {
  historical: "Vous consultez une lecture historique.",
  detached: "Retirée du contexte actif.",
  unavailable: "Source absente ou inaccessible dans l’outil.",
  connection_disabled: "La connexion est désactivée.",
  read_capability_disabled:
    "La lecture des sources existantes n’est plus autorisée.",
  connection_unverified:
    "Une nouvelle lecture autorisée est nécessaire avant de réutiliser cette source.",
  project_inactive: "Le projet est archivé.",
  invalid_identity:
    "L’identité de cet ancien reçu ne permet pas de le citer comme source.",
};
export function sourceTitle(title: string) {
  return title.trim() || "Sans titre";
}
export function sourceDate(value: string | null) {
  if (!value || !Number.isFinite(Date.parse(value))) return "Date indisponible";
  return new Intl.DateTimeFormat("fr-FR", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
export function safeSourceUrl(raw: string) {
  try {
    const url = new URL(raw);
    return url.protocol === "https:" && !url.username && !url.password
      ? url.href
      : null;
  } catch {
    return null;
  }
}
