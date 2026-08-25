import { createClient, type Session, type SupabaseClient } from "@supabase/supabase-js";
import { useEffect, useMemo, useState } from "react";

import { AuthContext, type AuthContextValue } from "./auth-context";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL as string | undefined;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY as
  | string
  | undefined;

const client =
  supabaseUrl && supabaseAnonKey
    ? createClient(supabaseUrl, supabaseAnonKey, {
        auth: {
          flowType: "pkce",
          persistSession: true,
          autoRefreshToken: true,
          detectSessionInUrl: false,
        },
      })
    : null;

let restorePromise: Promise<Session | null> | null = null;

function restoreSession(authClient: SupabaseClient) {
  restorePromise ??= (async () => {
    const code = new URL(window.location.href).searchParams.get("code");
    if (code) {
      const exchanged = await authClient.auth.exchangeCodeForSession(code);
      if (exchanged.error) throw exchanged.error;
      window.history.replaceState({}, "", "/auth/callback");
    }
    const current = await authClient.auth.getSession();
    if (current.error) throw current.error;
    return current.data.session;
  })();
  return restorePromise;
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [session, setSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(Boolean(client));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!client) return;
    let active = true;

    async function restore() {
      try {
        const restored = await restoreSession(client!);
        if (active) setSession(restored);
      } catch (reason) {
        if (active) {
          setError(
            reason instanceof Error
              ? reason.message
              : "Le lien de connexion n’a pas pu être vérifié.",
          );
        }
      } finally {
        if (active) setLoading(false);
      }
    }

    void restore();
    const { data } = client.auth.onAuthStateChange((_event, nextSession) => {
      setSession(nextSession);
      setError(null);
    });
    return () => {
      active = false;
      data.subscription.unsubscribe();
    };
  }, []);

  useEffect(() => {
    if (!client || loading) return;
    if (session?.access_token) {
      window.localStorage.setItem(
        "ai-center.access-token",
        session.access_token,
      );
    } else {
      window.localStorage.removeItem("ai-center.access-token");
      window.localStorage.removeItem("ai-center.workspace-id");
    }
  }, [session, loading]);

  const value = useMemo<AuthContextValue>(
    () => ({ enabled: Boolean(client), loading, session, client, error }),
    [loading, session, error],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
