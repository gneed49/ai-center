import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router";
import { ArrowRight, Building2, Mail } from "lucide-react";
import { api } from "@/api/client";
import { teamApi } from "@/api/team";
import type { InvitationPreview } from "@/api/team-types";
import { useAuth } from "@/auth/auth-context";
import {
  captureRequestContext,
  RequestContextChangedError,
} from "@/api/request-context";
import { invitationToken } from "@/lib/team-invitation";
import { formatDate } from "@/lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorState, LoadingState } from "@/components/app/page";

export function JoinCompanyPage() {
  const { invitationId = "" } = useParams();
  const location = useLocation();
  return (
    <JoinInvitation
      key={`${invitationId}:${location.hash}`}
      invitationId={invitationId}
      hash={location.hash}
    />
  );
}
function JoinInvitation({
  invitationId,
  hash,
}: {
  invitationId: string;
  hash: string;
}) {
  const auth = useAuth();
  const navigate = useNavigate();
  // Keep the received fragment until acceptance: identity changes intentionally remount App.
  // It is never written into browser storage, query keys, redirect URLs or API command records.
  const [token] = useState(() => invitationToken(hash));
  const [preview, setPreview] = useState<InvitationPreview | null>(null);
  const [loading, setLoading] = useState(Boolean(token));
  const [error, setError] = useState<Error | null>(null);
  const [acceptError, setAcceptError] = useState<Error | null>(null);
  const [displayName, setDisplayName] = useState("");
  const [accepting, setAccepting] = useState(false);
  const active = useRef(true);
  const workspaces = useQuery({
    queryKey: ["workspaces"],
    queryFn: api.workspaces,
    enabled: Boolean(auth.session),
  });
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    active.current = true;
    if (token) {
      void teamApi
        .preview(invitationId, token)
        .then((value) => {
          if (active.current) setPreview(value);
        })
        .catch((cause: unknown) => {
          if (active.current && !(cause instanceof RequestContextChangedError))
            setError(
              cause instanceof Error
                ? cause
                : new Error("L’invitation n’a pas pu être vérifiée."),
            );
        })
        .finally(() => {
          if (active.current) setLoading(false);
        });
    }
    return () => {
      active.current = false;
    };
  }, [invitationId, token, attempt]);
  const existing = workspaces.data?.find(
    (workspace) => workspace.public_id === preview?.workspace_public_id,
  );
  async function accept(event: FormEvent) {
    event.preventDefault();
    if (!token || !auth.session || !displayName.trim()) return;
    const context = captureRequestContext();
    setAccepting(true);
    setAcceptError(null);
    try {
      const workspace = await teamApi.accept(
        invitationId,
        token,
        displayName.trim(),
      );
      context.assertCurrent();
      auth.selectWorkspace(workspace.public_id);
      navigate("/", { replace: true });
    } catch (cause) {
      if (active.current && !(cause instanceof RequestContextChangedError))
        setAcceptError(
          cause instanceof Error
            ? cause
            : new Error("L’accès n’a pas pu être confirmé."),
        );
    } finally {
      if (active.current) setAccepting(false);
    }
  }
  return (
    <main className="grid min-h-dvh place-items-center bg-muted/30 px-4 py-12">
      <section className="w-full max-w-xl space-y-6 rounded-xl border bg-white p-6 shadow-sm sm:p-9">
        <div className="flex items-center gap-3">
          <span className="grid size-11 place-items-center rounded-lg bg-primary/10 text-primary">
            <Building2 />
          </span>
          <div>
            <p className="text-sm font-semibold text-primary">AI Center</p>
            <h1 className="text-2xl font-semibold tracking-tight">
              Rejoindre une entreprise
            </h1>
          </div>
        </div>
        {!token ? (
          <div role="alert" className="space-y-3">
            <h2 className="font-semibold">
              Le lien d’invitation est incomplet
            </h2>
            <p className="text-sm text-muted-foreground">
              Ouvrez le lien complet reçu de votre collègue. Si le problème
              persiste, demandez une nouvelle invitation.
            </p>
          </div>
        ) : loading ? (
          <LoadingState label="Vérification de l’invitation…" />
        ) : error ? (
          <ErrorState
            title="Invitation indisponible"
            error={error}
            retry={() => {
              setLoading(true);
              setError(null);
              setAttempt((value) => value + 1);
            }}
          />
        ) : preview ? (
          <>
            <div className="rounded-lg border bg-muted/20 p-5">
              <p className="text-xs font-medium text-muted-foreground">
                Vous êtes invité à rejoindre
              </p>
              <h2 className="mt-2 break-words text-xl font-semibold">
                {preview.company_name}
              </h2>
              <p className="mt-3 text-sm">
                Rôle :{" "}
                <strong>
                  {preview.role === "editor" ? "Collaborateur" : "Lecteur"}
                </strong>
              </p>
              <p className="mt-2 text-xs text-muted-foreground">
                Invitation valable jusqu’au{" "}
                {formatDate(preview.expires_at, true)}.
              </p>
            </div>
            {auth.loading ? (
              <LoadingState label="Restauration de votre connexion…" />
            ) : !auth.enabled ? (
              <p role="alert" className="text-sm text-amber-900">
                La connexion des membres n’est pas configurée sur cette
                instance. Contactez le propriétaire de l’entreprise.
              </p>
            ) : auth.session ? (
              <>
                {workspaces.isPending ? (
                  <LoadingState label="Vérification de vos accès…" />
                ) : workspaces.isError ? (
                  <ErrorState
                    error={workspaces.error}
                    retry={() => void workspaces.refetch()}
                  />
                ) : existing ? (
                  <div className="space-y-4">
                    <p className="text-sm text-muted-foreground">
                      Votre compte dispose déjà d’un accès à cette entreprise.
                    </p>
                    <Button
                      onClick={() => {
                        auth.selectWorkspace(existing.public_id);
                        navigate("/", { replace: true });
                      }}
                    >
                      Ouvrir {existing.name}
                      <ArrowRight />
                    </Button>
                  </div>
                ) : (
                  <form
                    onSubmit={(event) => void accept(event)}
                    className="space-y-4"
                  >
                    <p className="break-words text-sm text-muted-foreground">
                      Vous êtes connecté avec{" "}
                      {auth.session.user.email ?? "votre compte"}.
                    </p>
                    <div>
                      <label
                        htmlFor="join-display-name"
                        className="mb-2 block text-sm font-medium"
                      >
                        Votre nom dans l’équipe
                      </label>
                      <Input
                        id="join-display-name"
                        autoComplete="name"
                        value={displayName}
                        maxLength={120}
                        required
                        disabled={accepting}
                        onChange={(event) => setDisplayName(event.target.value)}
                      />
                    </div>
                    <p className="text-sm text-muted-foreground">
                      En rejoignant cette entreprise, vous accéderez à ses
                      projets selon le rôle indiqué.
                    </p>
                    <Button
                      type="submit"
                      disabled={accepting || !displayName.trim()}
                    >
                      {accepting
                        ? "Vérification de l’accès…"
                        : "Accepter l’invitation"}
                      <ArrowRight />
                    </Button>
                    {acceptError ? (
                      <ErrorState
                        title="L’accès n’a pas pu être confirmé"
                        error={acceptError}
                      />
                    ) : null}
                  </form>
                )}
              </>
            ) : (
              <InvitationLogin />
            )}
          </>
        ) : null}
        <div className="border-t pt-5">
          <Link
            to="/"
            className="text-sm text-muted-foreground underline underline-offset-4"
          >
            Revenir à AI Center
          </Link>
        </div>
      </section>
    </main>
  );
}
function InvitationLogin() {
  const auth = useAuth();
  const [email, setEmail] = useState("");
  const [pending, setPending] = useState(false);
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!auth.client || !email.trim()) return;
    setPending(true);
    setError(null);
    try {
      const result = await auth.client.auth.signInWithOtp({
        email: email.trim(),
        options: {
          shouldCreateUser: true,
          emailRedirectTo: `${window.location.origin}/auth/callback`,
        },
      });
      if (result.error) throw result.error;
      setSent(true);
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "Le lien de connexion n’a pas pu être demandé.",
      );
    } finally {
      setPending(false);
    }
  }
  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">Connectez-vous pour continuer</h2>
      <p className="text-sm text-muted-foreground">
        Un lien de connexion vous permettra de vérifier votre adresse et de
        créer votre compte si nécessaire. Il ne valide pas encore l’invitation.
      </p>
      {sent ? (
        <div
          className="space-y-2 rounded-lg bg-emerald-50 p-4 text-sm text-emerald-900"
          role="status"
        >
          <p>La demande de lien de connexion a été transmise.</p>
          <p>
            Gardez cet onglet ouvert. Ouvrez l’e-mail dans ce navigateur, puis
            revenez ici pour accepter l’invitation.
          </p>
        </div>
      ) : (
        <form onSubmit={(event) => void submit(event)} className="space-y-4">
          <label className="block text-sm font-medium" htmlFor="join-email">
            Votre adresse e-mail
          </label>
          <Input
            id="join-email"
            type="email"
            autoComplete="email"
            value={email}
            required
            disabled={pending}
            onChange={(event) => setEmail(event.target.value)}
          />
          <Button type="submit" disabled={pending}>
            <Mail />
            {pending ? "Demande en cours…" : "Recevoir mon lien de connexion"}
          </Button>
        </form>
      )}
      {error ? (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      ) : null}
    </div>
  );
}
