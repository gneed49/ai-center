import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import type { FormEvent } from "react";
import { artifactsApi } from "@/api/artifacts";
import { createIdempotencyKey } from "@/api/client";
import { artifactTypes } from "@/api/artifact-types";
import type {
  ArtifactDestination,
  ArtifactDestinations,
  DestinationProvider,
  SetArtifactDestination,
} from "@/api/artifact-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorState, LoadingState } from "@/components/app/page";
import { artifactLabels, providerLabels } from "./labels";

export function DestinationSettings({
  projectId,
  role,
}: {
  projectId?: string;
  role: "owner" | "editor" | "viewer";
}) {
  const cache = useQueryClient();
  const queryKey = ["artifact-destinations", projectId ?? "company"];
  const destinations = useQuery({
    queryKey,
    queryFn: () => artifactsApi.destinations(projectId),
  });
  const canEdit = projectId ? role !== "viewer" : role === "owner";
  if (destinations.isPending)
    return <LoadingState label="Chargement des destinations…" />;
  if (destinations.isError)
    return (
      <ErrorState
        error={destinations.error}
        retry={() => void destinations.refetch()}
      />
    );
  function saved(value: ArtifactDestinations) {
    cache.setQueryData(queryKey, value);
    void cache.invalidateQueries({ queryKey: ["artifact-destinations"] });
  }
  return (
    <section className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Destinations des livrables</h2>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
          {projectId
            ? "Les réglages de l’entreprise s’appliquent par défaut. Vous pouvez choisir une destination propre à ce projet."
            : "Choisissez la destination proposée pour chaque type de livrable. Les projets peuvent la remplacer."}
        </p>
        <p className="mt-2 text-sm text-amber-800">
          Ces réglages n’envoient aucun contenu. Une connexion autorisée et une
          action de publication explicite restent nécessaires pour transmettre
          un livrable à un outil externe.
        </p>
        {!canEdit ? (
          <p className="mt-2 text-sm text-muted-foreground">
            {projectId
              ? "Un éditeur peut modifier ces réglages."
              : "Seul le propriétaire peut modifier les destinations de l’entreprise."}
          </p>
        ) : null}
      </div>
      <div className="divide-y rounded-lg border">
        {artifactTypes.map((type) => {
          const destination = destinations.data.items.find(
            (item) => item.artifact_type === type,
          );
          return destination ? (
            <DestinationForm
              key={`${type}:${destination.revision}:${destination.provider}:${destination.target_id}`}
              value={destination}
              projectId={projectId}
              editable={canEdit}
              onSaved={saved}
            />
          ) : null;
        })}
      </div>
    </section>
  );
}

function DestinationForm({
  value,
  projectId,
  editable,
  onSaved,
}: {
  value: ArtifactDestination;
  projectId?: string;
  editable: boolean;
  onSaved: (value: ArtifactDestinations) => void;
}) {
  const [provider, setProvider] = useState<DestinationProvider>(value.provider);
  const [label, setLabel] = useState(value.label);
  const [target, setTarget] = useState(value.target_id ?? "");
  const previous = useRef<{ payload: string; key: string } | null>(null);
  const resetKey = useRef<string | null>(null);
  const save = useMutation({
    mutationFn: (input: SetArtifactDestination) => {
      const payload = JSON.stringify(input);
      if (previous.current?.payload !== payload)
        previous.current = { payload, key: createIdempotencyKey() };
      return artifactsApi.setDestination(
        projectId,
        input,
        previous.current.key,
      );
    },
    onSuccess: onSaved,
  });
  const reset = useMutation({
    mutationFn: () => {
      resetKey.current ??= createIdempotencyKey();
      return artifactsApi.resetDestination(
        projectId,
        value.artifact_type,
        value.revision,
        resetKey.current,
      );
    },
    onSuccess: onSaved,
  });
  const busy = save.isPending || reset.isPending;
  function submit(event: FormEvent) {
    event.preventDefault();
    save.mutate({
      artifact_type: value.artifact_type,
      provider,
      target_id: provider === "internal" ? null : target.trim(),
      label: label.trim(),
      expected_revision: value.revision,
    });
  }
  return (
    <form
      onSubmit={submit}
      className="space-y-4 p-5 sm:p-6"
      aria-label={`Destination ${artifactLabels[value.artifact_type]}`}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="font-medium">{artifactLabels[value.artifact_type]}</h3>
        <span className="text-xs text-muted-foreground">
          {value.origin === "company"
            ? "Réglage de l’entreprise"
            : value.origin === "project"
              ? "Réglage de ce projet"
              : "Stockage interne par défaut"}
        </span>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <div>
          <label
            htmlFor={`destination-provider-${value.artifact_type}`}
            className="mb-2 block text-sm"
          >
            Outil de destination
          </label>
          <select
            id={`destination-provider-${value.artifact_type}`}
            className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
            value={provider}
            disabled={!editable || busy}
            onChange={(event) =>
              setProvider(event.target.value as DestinationProvider)
            }
          >
            {Object.entries(providerLabels).map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label
            htmlFor={`destination-label-${value.artifact_type}`}
            className="mb-2 block text-sm"
          >
            Nom de la destination
          </label>
          <Input
            id={`destination-label-${value.artifact_type}`}
            value={label}
            maxLength={120}
            placeholder={
              provider === "internal"
                ? "Bibliothèque AI Center"
                : "Espace de travail de l’équipe"
            }
            disabled={!editable || busy}
            onChange={(event) => setLabel(event.target.value)}
          />
        </div>
      </div>
      {provider !== "internal" ? (
        <div>
          <label
            htmlFor={`destination-target-${value.artifact_type}`}
            className="mb-2 block text-sm"
          >
            {provider === "github"
              ? "Dépôt GitHub (organisation/dépôt)"
              : provider === "notion"
                ? "Identifiant de la page Notion"
                : "Identifiant de l’équipe Linear"}
          </label>
          <Input
            id={`destination-target-${value.artifact_type}`}
            value={target}
            required
            disabled={!editable || busy}
            onChange={(event) => setTarget(event.target.value)}
          />
          <p className="mt-2 text-xs text-muted-foreground">
            Référence de destination uniquement. N’entrez jamais de clé d’accès
            dans ce champ.
          </p>
        </div>
      ) : null}
      {editable ? (
        <div className="flex flex-wrap gap-2">
          <Button type="submit" variant="outline" disabled={busy}>
            {save.isPending ? "Enregistrement…" : "Enregistrer la destination"}
          </Button>
          {value.revision > 0 && (!projectId || value.origin === "project") ? (
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={() => reset.mutate()}
            >
              {projectId
                ? "Revenir au réglage de l’entreprise"
                : "Revenir au stockage interne"}
            </Button>
          ) : null}
        </div>
      ) : null}
      {save.error || reset.error ? (
        <ErrorState
          title="Le réglage n’a pas pu être confirmé"
          error={(save.error ?? reset.error)!}
        />
      ) : null}
    </form>
  );
}
