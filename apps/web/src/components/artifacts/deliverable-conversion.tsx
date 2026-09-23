import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { companyApi } from "@/api/company";
import { createIdempotencyKey } from "@/api/client";
import { draftStorageIdentity } from "@/api/request-context";
import { ErrorState } from "@/components/app/page";
import { Button } from "@/components/ui/button";

type Props = {
  projectId: string;
  deliverableId: string;
  version: number;
  archived: boolean;
};
export function DeliverableConversion(props: Props) {
  const identity = draftStorageIdentity();
  const storageKey = `ai-center.artifact-conversion:${identity.actorId}:${identity.workspaceId}:${props.projectId}:${props.deliverableId}`;
  return <Conversion key={storageKey} {...props} storageKey={storageKey} />;
}
function Conversion({
  projectId,
  deliverableId,
  version,
  archived,
  storageKey,
}: Props & { storageKey: string }) {
  const navigate = useNavigate();
  const cache = useQueryClient();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const [commandKey, setCommandKey] = useState(() => {
    const saved = localStorage.getItem(storageKey);
    return saved && /^[a-f0-9-]{36}$/i.test(saved) ? saved : null;
  });
  const receipt = useQuery({
    queryKey: ["artifact-conversion", projectId, commandKey],
    enabled: Boolean(commandKey),
    queryFn: () => artifactsApi.conversionReceipt(projectId, commandKey!),
    refetchInterval: (query) =>
      query.state.data?.status === "processing" ? 2000 : false,
  });
  const convert = useMutation({
    mutationFn: () => {
      const key = commandKey ?? createIdempotencyKey();
      // Opaque identity only, scoped to the signed-in actor and company.
      localStorage.setItem(storageKey, key);
      setCommandKey(key);
      return artifactsApi.fromDeliverable(projectId, deliverableId, key);
    },
    onSettled: () =>
      void cache.invalidateQueries({
        queryKey: ["artifact-conversion", projectId],
      }),
    onSuccess: (detail) => {
      void cache.invalidateQueries({ queryKey: ["artifacts", projectId] });
      cache.setQueryData(["artifact", detail.artifact.public_id], detail);
      navigate(`/artifacts/${detail.artifact.public_id}`);
    },
  });
  if (archived || !company.data || company.data.workspace.role === "viewer")
    return null;
  return (
    <section className="space-y-3 rounded-lg border bg-muted/30 p-5">
      <h2 className="font-semibold">Préparer ce livrable pour votre équipe</h2>
      <p className="text-sm text-muted-foreground">
        Copiez cette version {version} et ses sources dans un brouillon de la
        bibliothèque. Vous pourrez le modifier, le valider puis le publier dans
        votre outil. Le livrable d’origine reste conservé.
      </p>
      <Button
        disabled={
          convert.isPending || Boolean(commandKey && !receipt.data?.can_retry)
        }
        onClick={() => convert.mutate()}
      >
        {convert.isPending
          ? "Préparation…"
          : commandKey
            ? "Reprendre la conversion"
            : "Créer un brouillon depuis cette version"}
      </Button>
      {commandKey ? (
        <div className="space-y-3 text-sm" aria-label="Conversion sauvegardée">
          <p>
            {receipt.data?.status === "completed"
              ? "Votre brouillon est disponible."
              : receipt.data?.status === "processing"
                ? "Le serveur prépare votre brouillon. Cette demande reste disponible après rechargement."
                : receipt.data?.status === "expired"
                  ? "Le délai de reprise est dépassé. Consultez la bibliothèque avant de créer une nouvelle demande."
                  : "La demande est conservée. Vérifiez son état avant de la reprendre."}
          </p>
          {receipt.data?.result ? (
            <Button asChild>
              <Link to={`/artifacts/${receipt.data.result.artifact.public_id}`}>
                Ouvrir le brouillon retrouvé
              </Link>
            </Button>
          ) : null}
          <Button variant="outline" onClick={() => void receipt.refetch()}>
            Actualiser l’état de la conversion
          </Button>
          {receipt.data &&
          !["processing", "completed"].includes(receipt.data.status) &&
          !convert.isPending ? (
            <Button
              variant="ghost"
              onClick={() => {
                localStorage.removeItem(storageKey);
                setCommandKey(null);
              }}
            >
              Préparer une nouvelle conversion
            </Button>
          ) : null}
          {receipt.error ? (
            <p role="alert">
              L’état est indisponible. Votre demande reste conservée.
            </p>
          ) : null}
        </div>
      ) : null}
      {convert.error ? (
        <ErrorState
          error={convert.error}
          title="La conversion n’a pas pu être confirmée"
        />
      ) : null}
    </section>
  );
}
