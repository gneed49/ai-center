import { useEffect, useState } from "react";
import { ExternalLink } from "lucide-react";
import { providersApi } from "@/api/providers";
import type {
  SubscriptionLogin,
  SubscriptionStatus,
} from "@/api/provider-types";
import { Button } from "@/components/ui/button";
import { FieldError } from "@/components/ui/field";
import { officialLoginUrl, providerError } from "@/lib/provider-settings";
import { useProviderAction } from "./use-provider-action";
import {
  openSubscriptionLink,
  usesSystemBrowser,
} from "@/lib/subscription-link";

const loginLabels = {
  pending: "Connexion en attente…",
  connected: "Abonnement connecté.",
  failed: "La connexion n’a pas abouti. Vous pouvez recommencer.",
  cancelled: "Connexion annulée.",
  expired: "La connexion a expiré. Vous pouvez recommencer.",
};

export function SubscriptionPanel({
  connectionId,
  available,
}: {
  connectionId: string;
  available: boolean;
}) {
  const [status, setStatus] = useState<SubscriptionStatus>();
  // A login URL is transient state: never put it in query caches or browser storage.
  const [login, setLogin] = useState<SubscriptionLogin>();
  const [pollError, setPollError] = useState<string>();
  const [revision, setRevision] = useState(0);
  const action = useProviderAction();
  const pending = login?.status === "pending";
  const unsupportedAccount =
    status?.authenticated === true && !status.connected;
  const loginId = pending ? login.login_id : undefined;

  useEffect(() => {
    if (!available) return;
    const controller = new AbortController();
    void providersApi
      .subscriptionStatus(connectionId, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setStatus(value);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) setPollError(providerError(cause));
      });
    return () => controller.abort();
  }, [connectionId, available, revision]);

  useEffect(() => {
    if (!loginId) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const value = await providersApi.loginStatus(
          connectionId,
          loginId!,
          controller.signal,
        );
        if (controller.signal.aborted) return;
        setPollError(undefined);
        setLogin(
          value.status === "pending"
            ? value
            : { ...value, auth_url: undefined },
        );
        if (value.status === "pending")
          timer = setTimeout(() => void poll(), 2000);
        else setRevision((current) => current + 1);
      } catch (cause) {
        if (!controller.signal.aborted) {
          setPollError(providerError(cause));
          // Keep the cancel action available, but do not keep retrying indefinitely.
          setLogin((current) =>
            current ? { ...current, auth_url: undefined } : current,
          );
        }
      }
    }
    timer = setTimeout(() => void poll(), 1000);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [connectionId, loginId]);

  function start() {
    setPollError(undefined);
    setLogin(undefined);
    void action.run(
      () => providersApi.startLogin(connectionId),
      (value) => {
        setLogin(
          value.status === "pending"
            ? value
            : { ...value, auth_url: undefined },
        );
        if (value.status === "connected") setRevision((current) => current + 1);
      },
    );
  }
  const url = pending ? officialLoginUrl(login.auth_url) : undefined;
  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-muted-foreground" role="status">
        {unsupportedAccount && !pending
          ? status.message
          : login
            ? loginLabels[login.status]
            : status?.connected
              ? "Abonnement connecté."
              : (status?.message ??
                "Connectez votre abonnement personnel avec le client officiel.")}
      </p>
      {url && (
        <Button asChild variant="outline">
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(event) => {
              if (usesSystemBrowser()) {
                event.preventDefault();
                void action.run(
                  () => openSubscriptionLink(url),
                  () => {},
                );
              }
            }}
          >
            Continuer chez le fournisseur
            <ExternalLink data-icon="inline-end" />
          </a>
        </Button>
      )}
      <div className="flex flex-wrap gap-2">
        {pending ? (
          <Button
            type="button"
            variant="outline"
            disabled={action.busy}
            onClick={() => {
              void action.run(
                () => providersApi.cancelLogin(connectionId, login.login_id),
                (value) => setLogin({ ...value, auth_url: undefined }),
              );
            }}
          >
            Annuler la connexion
          </Button>
        ) : (
          <Button
            type="button"
            variant="outline"
            disabled={
              !available ||
              (status?.available === false && !unsupportedAccount) ||
              action.busy
            }
            onClick={start}
          >
            {unsupportedAccount
              ? "Changer de compte"
              : status?.connected
                ? "Reconnecter l’abonnement"
                : "Connecter mon abonnement"}
          </Button>
        )}
        {status?.authenticated === true && !pending && (
          <Button
            type="button"
            variant="outline"
            disabled={action.busy}
            onClick={() => {
              void action.run(
                () => providersApi.logout(connectionId),
                (value) => {
                  setStatus(value);
                  setLogin(undefined);
                },
              );
            }}
          >
            Déconnecter l’abonnement
          </Button>
        )}
        {pollError && !pending && (
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setPollError(undefined);
              setRevision((current) => current + 1);
            }}
          >
            Actualiser l’état
          </Button>
        )}
      </div>
      {(action.error || pollError) && (
        <FieldError>{action.error ?? pollError}</FieldError>
      )}
    </div>
  );
}
