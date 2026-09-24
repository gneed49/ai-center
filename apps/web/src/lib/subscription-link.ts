import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { officialLoginUrl } from "./provider-settings";

export function usesSystemBrowser() {
  return isTauri();
}

/** Called only by an explicit click; errors must never contain the OAuth URL. */
export async function openSubscriptionLink(value: string) {
  const url = officialLoginUrl(value);
  if (!url) throw new Error("Lien de connexion fournisseur invalide.");
  try {
    await openUrl(url);
  } catch {
    throw new Error("Impossible d’ouvrir le navigateur pour cette connexion.");
  }
}
