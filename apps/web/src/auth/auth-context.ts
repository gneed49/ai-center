import type { Session, SupabaseClient } from "@supabase/supabase-js";
import { createContext, useContext } from "react";

export interface AuthContextValue {
  enabled: boolean;
  loading: boolean;
  session: Session | null;
  workspaceId: string | null;
  selectWorkspace: (workspaceId: string) => void;
  client: SupabaseClient | null;
  error: string | null;
}

export const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth() {
  const value = useContext(AuthContext);
  if (!value) throw new Error("useAuth must be used inside AuthProvider");
  return value;
}
