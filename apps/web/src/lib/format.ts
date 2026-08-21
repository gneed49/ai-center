export function formatDate(value: string, withTime = false) {
  return new Intl.DateTimeFormat("fr-FR", {
    day: "2-digit",
    month: "short",
    ...(withTime ? { hour: "2-digit", minute: "2-digit" } : {}),
  }).format(new Date(value));
}

export function shortId(value: string) {
  return value.slice(0, 8);
}

export function humanize(value: string) {
  return value.replaceAll("_", " ").replaceAll(".", " · ");
}
