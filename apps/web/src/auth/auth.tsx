import {
  createClient,
  type Session,
  type SupabaseClient,
} from "@supabase/supabase-js";
import { QueryClientProvider } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { setRequestIdentity } from "@/api/request-context";
import { createQueryClient } from "./query-client";

import { AuthContext, type AuthContextValue } from "./auth-context";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL as string | undefined;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY as
  string | undefined;

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

function workspaceKey(actorId: string) {
  return `ai-center.workspace.v1:${actorId}`;
}

function savedWorkspace(actorId: string | undefined) {
  if (!actorId) return null;
  try {
    return window.localStorage.getItem(workspaceKey(actorId));
  } catch {
    return null;
  }
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState(() => ({
    session: null as Session | null,
    workspaceId: null as string | null,
    loading: Boolean(client),
    error: null as string | null,
    revision: 0,
    queryClient: createQueryClient(),
  }));
  const current = useRef(state);

  const publish = useCallback(
    (session: Session | null, workspaceId: string | null) => {
      const previous = current.current;
      const changed = setRequestIdentity(
        session?.user.id ?? null,
        workspaceId,
        session?.access_token ?? null,
      );
      if (changed) {
        // In-flight reads and cached mutations belong to the previous identity.
        void previous.queryClient.cancelQueries();
        previous.queryClient.clear();
      }
      const next = {
        session,
        workspaceId,
        loading: false,
        error: null,
        revision: previous.revision + (changed ? 1 : 0),
        queryClient: changed ? createQueryClient() : previous.queryClient,
      };
      current.current = next;
      setState(next);
    },
    [],
  );

  const selectWorkspace = useCallback(
    (workspaceId: string) => {
      const session = current.current.session;
      if (!session) return;
      try {
        window.localStorage.setItem(workspaceKey(session.user.id), workspaceId);
      } catch {
        // Selection remains valid for this tab when persistence is unavailable.
      }
      publish(session, workspaceId);
    },
    [publish],
  );

  useEffect(() => {
    if (!client) return;
    let active = true;
    let authGeneration = 0;
    // Never reuse the legacy, unowned token/workspace pair in an Auth session.
    setRequestIdentity(null, null, null);
    try {
      window.localStorage.removeItem("ai-center.access-token");
      window.localStorage.removeItem("ai-center.workspace-id");
    } catch {
      // The managed request context never reads this legacy storage.
    }

    function applySession(session: Session | null) {
      if (!active) return;
      const previous = current.current;
      const workspaceId =
        previous.session?.user.id === session?.user.id
          ? previous.workspaceId
          : savedWorkspace(session?.user.id);
      publish(session, workspaceId);
    }

    async function restore() {
      const generation = authGeneration;
      try {
        const restored = await restoreSession(client!);
        if (active && generation === authGeneration) applySession(restored);
      } catch (reason) {
        if (active && !current.current.session) {
          const next = {
            ...current.current,
            loading: false,
            error:
              reason instanceof Error
                ? reason.message
                : "Le lien de connexion n’a pas pu être vérifié.",
          };
          current.current = next;
          setState(next);
        }
      }
    }

    const { data } = client.auth.onAuthStateChange((event, nextSession) => {
      if (event !== "INITIAL_SESSION") authGeneration += 1;
      if (event === "INITIAL_SESSION" && !nextSession && current.current.error)
        return;
      applySession(nextSession);
    });
    void restore();
    function selectionChanged(event: StorageEvent) {
      const session = current.current.session;
      if (session && event.key === workspaceKey(session.user.id)) {
        publish(session, event.newValue);
      }
    }
    window.addEventListener("storage", selectionChanged);
    return () => {
      active = false;
      data.subscription.unsubscribe();
      window.removeEventListener("storage", selectionChanged);
    };
  }, [publish]);

  const value = useMemo<AuthContextValue>(
    () => ({
      enabled: Boolean(client),
      loading: state.loading,
      session: state.session,
      workspaceId: state.workspaceId,
      selectWorkspace,
      client,
      error: state.error,
    }),
    [
      state.loading,
      state.session,
      state.workspaceId,
      state.error,
      selectWorkspace,
    ],
  );

  return (
    <AuthContext.Provider value={value}>
      <QueryClientProvider key={state.revision} client={state.queryClient}>
        {children}
      </QueryClientProvider>
    </AuthContext.Provider>
  );
}
