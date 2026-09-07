import { ApiError } from "@/api/client";

/** Never echo a transport error that could contain a credential or auth URL. */
export function providerError(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 401) return "Renouvelez votre connexion à AI Center.";
    if (error.status === 403)
      return "Votre rôle ne permet pas de modifier ces réglages.";
    if (error.status === 409)
      return "Ce réglage a changé. Actualisez la page avant de réessayer.";
    if (error.status === 429)
      return "Le fournisseur limite les demandes. Réessayez dans quelques instants.";
    if (error.status === 0)
      return "Le serveur est injoignable. Vous pouvez réessayer après reconnexion.";
  }
  return "L’action a échoué. Vérifiez la connexion, le modèle et la disponibilité du fournisseur.";
}

export function officialLoginUrl(
  value: string | null | undefined,
): string | undefined {
  if (!value) return;
  try {
    const url = new URL(value);
    const paths: Record<string, string> = {
      "claude.com": "/cai/oauth/authorize",
      "claude.ai": "/oauth/authorize",
      "platform.claude.com": "/oauth/authorize",
      "console.anthropic.com": "/oauth/authorize",
    };
    if (
      url.protocol === "https:" &&
      !url.username &&
      !url.password &&
      !url.port &&
      !url.hash &&
      paths[url.hostname] === url.pathname.replace(/\/$/, "") &&
      !["access_token", "refresh_token", "id_token"].some((key) =>
        url.searchParams.has(key),
      ) &&
      url.searchParams.getAll("code").every((value) => value === "true")
    )
      return url.href;
  } catch {
    /* Invalid links never become clickable. */
  }
}

export interface RetryIdentity {
  fingerprint: string;
  key: string;
}

/** Retain only a digest between retries, never the submitted secret payload. */
export async function retryIdentity(
  payload: unknown,
  previous?: RetryIdentity,
): Promise<RetryIdentity> {
  const bytes = new TextEncoder().encode(JSON.stringify(payload));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  bytes.fill(0);
  const fingerprint = Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  return previous?.fingerprint === fingerprint
    ? previous
    : { fingerprint, key: crypto.randomUUID() };
}
