import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { artifactsApi } from "@/api/artifacts";
import { createIdempotencyKey } from "@/api/client";
import type { ArtifactType } from "@/api/artifact-types";
import type { SessionSummary } from "@/api/types";
import { ErrorState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  generationStorageKey,
  readGenerationCommand,
  saveGenerationCommand,
} from "./generation-command";
import type { GenerationCommand } from "./generation-command";
import { artifactLabels } from "./labels";

type GeneratorProps = {
  projectId: string;
  type: ArtifactType;
  sessions: SessionSummary[];
  selectedSessionId?: string;
};
export function ArtifactGenerator(props: GeneratorProps) {
  const storageKey = generationStorageKey(props.projectId, props.type);
  return <Generator key={storageKey} {...props} storageKey={storageKey} />;
}
function Generator({
  projectId,
  type,
  sessions,
  selectedSessionId,
  storageKey,
}: GeneratorProps & { storageKey: string }) {
  const navigate = useNavigate();
  const cache = useQueryClient();
  const technical = type === "technical_plan" || type === "technical_tickets";
  const eligible = sessions.filter((session) =>
    technical
      ? ["tech", "dev"].includes(session.scope_kind)
      : ["general", "product", "sales"].includes(session.scope_kind),
  );
  const [saved, setSaved] = useState(() =>
    readGenerationCommand(storageKey, type),
  );
  const [chosen, setChosen] = useState(
    saved?.input.session_id ?? selectedSessionId ?? "",
  );
  const sessionId = eligible.some((session) => session.public_id === chosen)
    ? chosen
    : "";
  const [instructions, setInstructions] = useState(
    saved?.input.instructions ?? "",
  );
  const receipt = useQuery({
    queryKey: ["artifact-generation", projectId, saved?.key],
    enabled: Boolean(saved),
    queryFn: () => artifactsApi.generationReceipt(projectId, saved!.key),
    refetchInterval: (query) =>
      query.state.data?.status === "processing" ? 2000 : false,
  });
  const [restoredAt] = useState(Date.now);
  const expired = Boolean(
    saved && restoredAt - saved.createdAt > 23 * 60 * 60 * 1000,
  );
  const recoverable = Boolean(saved && !expired && receipt.data?.can_retry);

  const generate = useMutation({
    mutationFn: () => {
      const command: GenerationCommand = saved ?? {
        key: createIdempotencyKey(),
        createdAt: Date.now(),
        input: {
          artifact_type: type,
          session_id: sessionId,
          instructions: instructions.trim(),
        },
      };
      saveGenerationCommand(storageKey, command);
      setSaved(command);
      return artifactsApi.generate(projectId, command.input, command.key);
    },
    onSettled: () =>
      void cache.invalidateQueries({
        queryKey: ["artifact-generation", projectId],
      }),
    onSuccess: (detail) => {
      localStorage.removeItem(storageKey);
      void cache.invalidateQueries({ queryKey: ["artifacts", projectId] });
      cache.setQueryData(["artifact", detail.artifact.public_id], detail);
      navigate(`/artifacts/${detail.artifact.public_id}`);
    },
  });
  return (
    <section
      className="space-y-4 rounded-lg border bg-muted/30 p-5"
      aria-label="Préparation avec un agent"
    >
      <h3 className="font-semibold">Demander un brouillon à un agent</h3>
      <p className="text-sm text-muted-foreground">
        L’agent prépare un brouillon structuré (
        {artifactLabels[type].toLocaleLowerCase()}) à partir de cette
        conversation et de ses sources. Vous pourrez la modifier et la valider
        avant toute publication.
      </p>
      <label className="block space-y-2 text-sm font-medium">
        Conversation de départ
        <select
          className="min-h-11 w-full rounded-md border bg-white px-3"
          value={sessionId}
          onChange={(event) => setChosen(event.target.value)}
          disabled={generate.isPending || Boolean(saved)}
        >
          <option value="">Choisir une conversation</option>
          {eligible.map((session) => (
            <option key={session.public_id} value={session.public_id}>
              {session.title}
            </option>
          ))}
        </select>
      </label>
      {!eligible.length ? (
        <p className="text-sm text-muted-foreground">
          {technical
            ? "Préparez une transmission vers le lead technique ou le développeur pour commencer."
            : "Commencez une conversation avec un agent du projet, puis revenez préparer le document."}
        </p>
      ) : null}
      <label className="block space-y-2 text-sm font-medium">
        Ce que le document doit préparer
        <Textarea
          value={instructions}
          onChange={(event) => setInstructions(event.target.value)}
          maxLength={8000}
          disabled={generate.isPending || Boolean(saved)}
          placeholder="Le public, le résultat attendu et les points à approfondir…"
        />
      </label>
      <Button
        type="button"
        disabled={
          !sessionId ||
          !instructions.trim() ||
          generate.isPending ||
          Boolean(saved && !recoverable)
        }
        onClick={() => generate.mutate()}
      >
        {generate.isPending
          ? "Préparation du brouillon…"
          : saved
            ? "Reprendre la même demande"
            : "Générer le brouillon avec l’agent"}
      </Button>
      {saved ? (
        <div
          className="space-y-3 rounded-md border p-4 text-sm"
          aria-label="Demande sauvegardée"
        >
          <p>
            {receipt.data?.status === "completed"
              ? "Votre brouillon est disponible."
              : expired || receipt.data?.status === "expired"
                ? "Le délai de reprise est dépassé. Consultez la bibliothèque avant une nouvelle demande."
                : receipt.data?.status === "processing"
                  ? "Le serveur prépare votre brouillon. Vous pouvez revenir sur cette page ; aucune génération supplémentaire ne sera lancée."
                  : "Votre demande est conservée dans ce navigateur. Consultez son état avant de la reprendre."}
          </p>
          {receipt.data?.result ? (
            <Button asChild>
              <Link to={`/artifacts/${receipt.data.result.artifact.public_id}`}>
                Ouvrir le brouillon retrouvé
              </Link>
            </Button>
          ) : null}
          <Button
            type="button"
            variant="outline"
            onClick={() => void receipt.refetch()}
          >
            Actualiser l’état de la demande
          </Button>
          {receipt.data?.status !== "processing" && !generate.isPending ? (
            <Button
              type="button"
              variant="ghost"
              onClick={() => {
                localStorage.removeItem(storageKey);
                setSaved(null);
              }}
            >
              Préparer une nouvelle demande
            </Button>
          ) : null}
          {receipt.error ? (
            <p role="alert">
              L’état est indisponible. Votre demande reste conservée.
            </p>
          ) : null}
        </div>
      ) : null}
      {generate.error ? (
        <ErrorState
          title="Le brouillon n’a pas pu être confirmé"
          error={generate.error}
        />
      ) : null}
    </section>
  );
}
