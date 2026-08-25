import { useQuery } from "@tanstack/react-query";
import { ArrowRight, KeyRound, LoaderCircle } from "lucide-react";
import { type FormEvent, useEffect, useState } from "react";

import { api } from "@/api/client";
import { Button } from "@/components/ui/button";
import { useAuth } from "./auth-context";

export function LoginPage() {
  const { client, error: callbackError } = useAuth();
  const [email, setEmail] = useState("");
  const [pending, setPending] = useState(false);
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!client || !email.trim()) return;
    setPending(true);
    setError(null);
    const result = await client.auth.signInWithOtp({
      email: email.trim(),
      options: {
        emailRedirectTo: `${window.location.origin}/auth/callback`,
        shouldCreateUser: false,
      },
    });
    setPending(false);
    if (result.error) setError(result.error.message);
    else setSent(true);
  }

  return (
    <main className="grid min-h-dvh place-items-center bg-[#11182b] px-6 py-12 text-white">
      <section className="w-full max-w-md border border-white/10 bg-white/[0.06] p-8 shadow-2xl">
        <div className="mb-8 grid size-11 place-items-center bg-indigo-400/15 text-indigo-200">
          <KeyRound aria-hidden className="size-5" />
        </div>
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-indigo-200">
          Alpha privée
        </p>
        <h1 className="mt-3 text-3xl font-semibold tracking-tight">AI Center</h1>
        <p className="mt-3 text-sm leading-6 text-slate-300">
          Connectez-vous par lien magique pour accéder au plan de contrôle de
          contexte de votre workspace.
        </p>
        {sent ? (
          <div className="mt-8 border border-emerald-300/25 bg-emerald-300/10 p-4 text-sm text-emerald-100" role="status">
            Le lien a été envoyé. Vous pouvez fermer cet onglet après avoir
            ouvert l’e-mail.
          </div>
        ) : (
          <form className="mt-8 space-y-4" onSubmit={submit}>
            <label className="block text-sm font-medium" htmlFor="login-email">
              Adresse e-mail
            </label>
            <input
              id="login-email"
              type="email"
              autoComplete="email"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className="min-h-11 w-full border border-white/20 bg-slate-950/30 px-3 text-white outline-none focus:border-indigo-300"
            />
            <Button className="min-h-11 w-full" disabled={pending} type="submit">
              {pending ? <LoaderCircle className="animate-spin" /> : <ArrowRight />}
              Recevoir le lien magique
            </Button>
          </form>
        )}
        {error ?? callbackError ? (
          <p className="mt-4 text-sm text-rose-200" role="alert">
            {error ?? callbackError}
          </p>
        ) : null}
      </section>
    </main>
  );
}

export function AuthCallbackPage() {
  const { session, error } = useAuth();
  useEffect(() => {
    if (session) window.location.replace("/");
  }, [session]);
  return (
    <main className="grid min-h-dvh place-items-center bg-[#11182b] px-6 text-white">
      <div className="text-center" role="status" aria-live="polite">
        <LoaderCircle className="mx-auto size-7 animate-spin text-indigo-300" />
        <p className="mt-4 text-sm text-slate-300">
          {error ?? "Vérification du lien sécurisé…"}
        </p>
      </div>
    </main>
  );
}

export function WorkspacePicker() {
  const { client } = useAuth();
  const workspaces = useQuery({
    queryKey: ["workspaces"],
    queryFn: api.workspaces,
  });

  return (
    <main className="grid min-h-dvh place-items-center bg-[#f7f8fb] px-6 py-12">
      <section className="w-full max-w-xl border border-slate-200 bg-white p-8 shadow-sm">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-indigo-600">
          Workspace
        </p>
        <h1 className="mt-3 text-2xl font-semibold">Choisir le contexte partagé</h1>
        {workspaces.isLoading ? (
          <p className="mt-6 text-sm text-slate-500" role="status">
            Chargement des accès…
          </p>
        ) : workspaces.error ? (
          <div className="mt-6" role="alert">
            <p className="text-sm text-rose-700">{workspaces.error.message}</p>
            <Button className="mt-4" onClick={() => workspaces.refetch()}>
              Réessayer
            </Button>
          </div>
        ) : workspaces.data?.length ? (
          <div className="mt-6 grid gap-3">
            {workspaces.data.map((workspace) => (
              <button
                key={workspace.public_id}
                type="button"
                className="flex min-h-16 items-center justify-between border border-slate-200 px-4 text-left hover:border-indigo-300 hover:bg-indigo-50"
                onClick={() => {
                  window.localStorage.setItem(
                    "ai-center.workspace-id",
                    workspace.public_id,
                  );
                  window.location.replace("/");
                }}
              >
                <span className="font-medium">{workspace.name}</span>
                <span className="text-xs uppercase tracking-wide text-slate-500">
                  {workspace.role}
                </span>
              </button>
            ))}
          </div>
        ) : (
          <p className="mt-6 text-sm text-amber-800" role="alert">
            Aucun workspace accepté n’est associé à ce compte. Demandez une
            invitation au propriétaire.
          </p>
        )}
        <Button
          className="mt-6"
          variant="outline"
          onClick={() => void client?.auth.signOut()}
        >
          Se déconnecter
        </Button>
      </section>
    </main>
  );
}
