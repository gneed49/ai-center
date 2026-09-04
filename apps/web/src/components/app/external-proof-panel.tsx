import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  Check,
  ExternalLink,
  GitPullRequest,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  X,
} from "lucide-react";
import { type FormEvent, useState } from "react";

import { api, createIdempotencyKey } from "@/api/client";
import type {
  CoverageItem,
  ExternalEvidence,
  ExternalReferenceView,
  UUID,
} from "@/api/types";
import { ErrorState } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { shortId } from "@/lib/format";

interface ExternalProofPanelProps {
  projectId: UUID;
  deliverableId: UUID;
  coverageItems: CoverageItem[];
}

interface ImportCommand {
  url: string;
  idempotencyKey: string;
}

interface ReferenceCommand {
  referenceId: UUID;
  idempotencyKey: string;
}

interface EvidenceCommand extends ReferenceCommand {
  coverage: CoverageItem;
}

interface ReviewCommand extends ReferenceCommand {
  evidenceId: UUID;
  decision: "validate" | "reject";
}

export function ExternalProofPanel({
  projectId,
  deliverableId,
  coverageItems,
}: ExternalProofPanelProps) {
  const queryClient = useQueryClient();
  const [url, setUrl] = useState("");
  const references = useQuery({
    queryKey: ["external-references", projectId],
    queryFn: () => api.externalReferences(projectId),
  });
  const details = useQueries({
    queries: (references.data ?? []).map((reference) => ({
      queryKey: ["external-reference", reference.public_id],
      queryFn: () => api.externalReference(reference.public_id),
    })),
  });

  const refreshProjection = async (referenceId?: UUID) => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: ["external-references", projectId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["coverage", projectId],
      }),
      queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] }),
      ...(referenceId
        ? [
            queryClient.invalidateQueries({
              queryKey: ["external-reference", referenceId],
            }),
          ]
        : []),
    ]);
  };

  const importReference = useMutation({
    mutationFn: (command: ImportCommand) =>
      api.createExternalReference(
        projectId,
        command.url,
        command.idempotencyKey,
      ),
    onSuccess: async (reference) => {
      setUrl("");
      await refreshProjection(reference.public_id);
    },
  });
  const refreshReference = useMutation({
    mutationFn: (command: ReferenceCommand) =>
      api.refreshExternalReference(command.referenceId, command.idempotencyKey),
    onSuccess: (reference) => refreshProjection(reference.public_id),
  });
  const createEvidence = useMutation({
    mutationFn: (command: EvidenceCommand) => {
      if (!command.coverage.section_public_id)
        throw new Error(
          "Le livrable doit exposer une section durable avant de créer une preuve.",
        );
      return api.createExternalEvidence(
        command.referenceId,
        {
          requirement_id: command.coverage.requirement_public_id,
          deliverable_id: deliverableId,
          deliverable_section_id: command.coverage.section_public_id,
          title: `Référence observée — ${command.coverage.requirement_title}`,
          description:
            "Preuve candidate issue d’une observation GitHub read-only.",
        },
        command.idempotencyKey,
      );
    },
    onSuccess: (_evidence, command) => refreshProjection(command.referenceId),
  });
  const reviewEvidence = useMutation({
    mutationFn: (command: ReviewCommand) =>
      api.reviewExternalEvidence(
        command.referenceId,
        command.evidenceId,
        command.decision,
        command.idempotencyKey,
      ),
    onSuccess: (_evidence, command) => refreshProjection(command.referenceId),
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    const canonicalUrl = url.trim();
    if (!canonicalUrl || importReference.isPending) return;
    importReference.mutate({
      url: canonicalUrl,
      idempotencyKey: createIdempotencyKey(),
    });
  }

  return (
    <section
      className="border border-slate-200 bg-white"
      aria-labelledby="external-proof-title"
    >
      <div className="border-b border-slate-200 p-5 sm:p-6">
        <div className="flex items-center gap-2">
          <GitPullRequest className="size-4 text-indigo-600" aria-hidden />
          <h2 id="external-proof-title" className="font-semibold">
            Preuves GitHub observées
          </h2>
        </div>
        <p className="mt-2 text-sm leading-6 text-slate-600">
          Observez un dépôt, une pull request ou un commit GitHub. Les preuves
          portent sur un SHA précis et attendent votre validation humaine.
        </p>
        <form
          className="mt-5 flex flex-col gap-3 sm:flex-row"
          onSubmit={submit}
        >
          <div className="min-w-0 flex-1">
            <label className="sr-only" htmlFor="github-reference-url">
              URL canonique du dépôt, de la pull request ou du commit GitHub
            </label>
            <input
              id="github-reference-url"
              type="url"
              required
              placeholder="https://github.com/owner/repository/pull/123"
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              className="min-h-11 w-full border border-slate-300 px-3 text-sm outline-none focus:border-indigo-500"
            />
          </div>
          <Button disabled={importReference.isPending} type="submit">
            {importReference.isPending ? (
              <LoaderCircle className="animate-spin" aria-hidden />
            ) : (
              <ExternalLink aria-hidden />
            )}
            {importReference.isPending
              ? "Observation…"
              : "Observer la référence"}
          </Button>
        </form>
        {importReference.error ? (
          <PersistentMutationError
            error={importReference.error}
            retry={
              importReference.variables
                ? () => importReference.mutate(importReference.variables)
                : undefined
            }
          />
        ) : null}
      </div>

      <div className="space-y-4 p-5 sm:p-6" aria-live="polite">
        {references.isLoading ? (
          <p className="text-sm text-slate-500" role="status">
            Chargement des références externes…
          </p>
        ) : references.error ? (
          <ErrorState
            error={references.error}
            title="Les références GitHub sont indisponibles"
            retry={() => references.refetch()}
          />
        ) : references.data?.length ? (
          references.data.map((summary, index) => {
            const detailQuery = details[index];
            const reference = detailQuery?.data;
            return (
              <ReferenceCard
                key={summary.public_id}
                reference={reference}
                fallback={summary}
                loading={Boolean(detailQuery?.isLoading)}
                error={detailQuery?.error ?? null}
                coverageItems={coverageItems}
                refreshPending={
                  refreshReference.isPending &&
                  refreshReference.variables?.referenceId === summary.public_id
                }
                evidencePending={
                  createEvidence.isPending &&
                  createEvidence.variables?.referenceId === summary.public_id
                }
                reviewPending={
                  reviewEvidence.isPending &&
                  reviewEvidence.variables?.referenceId === summary.public_id
                }
                onRefresh={() =>
                  refreshReference.mutate({
                    referenceId: summary.public_id,
                    idempotencyKey: createIdempotencyKey(),
                  })
                }
                onCreateEvidence={(coverage) =>
                  createEvidence.mutate({
                    referenceId: summary.public_id,
                    coverage,
                    idempotencyKey: createIdempotencyKey(),
                  })
                }
                onReview={(evidenceId, decision) =>
                  reviewEvidence.mutate({
                    referenceId: summary.public_id,
                    evidenceId,
                    decision,
                    idempotencyKey: createIdempotencyKey(),
                  })
                }
              />
            );
          })
        ) : (
          <p className="text-sm text-slate-500">
            Aucune référence GitHub n’est encore rattachée à ce projet.
          </p>
        )}
        {refreshReference.error ? (
          <PersistentMutationError
            error={refreshReference.error}
            retry={
              refreshReference.variables
                ? () => refreshReference.mutate(refreshReference.variables)
                : undefined
            }
          />
        ) : null}
        {createEvidence.error ? (
          <PersistentMutationError
            error={createEvidence.error}
            retry={
              createEvidence.variables
                ? () => createEvidence.mutate(createEvidence.variables)
                : undefined
            }
          />
        ) : null}
        {reviewEvidence.error ? (
          <PersistentMutationError
            error={reviewEvidence.error}
            retry={
              reviewEvidence.variables
                ? () => reviewEvidence.mutate(reviewEvidence.variables)
                : undefined
            }
          />
        ) : null}
      </div>
    </section>
  );
}

function ReferenceCard({
  reference,
  fallback,
  loading,
  error,
  coverageItems,
  refreshPending,
  evidencePending,
  reviewPending,
  onRefresh,
  onCreateEvidence,
  onReview,
}: {
  reference?: ExternalReferenceView;
  fallback: ExternalReferenceView | Omit<ExternalReferenceView, "evidences">;
  loading: boolean;
  error: Error | null;
  coverageItems: CoverageItem[];
  refreshPending: boolean;
  evidencePending: boolean;
  reviewPending: boolean;
  onRefresh: () => void;
  onCreateEvidence: (coverage: CoverageItem) => void;
  onReview: (evidenceId: UUID, decision: "validate" | "reject") => void;
}) {
  const evidences = reference?.evidences ?? [];
  const headSha = reference?.latest_observation?.observed_state.head_sha;
  return (
    <article className="border border-slate-200 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <a
              className="break-all font-semibold text-indigo-700 underline-offset-4 hover:underline"
              href={fallback.canonical_url}
              target="_blank"
              rel="noreferrer"
            >
              {fallback.display_title}
            </a>
            <StatusPill status={fallback.sync_status} />
          </div>
          <p className="mt-1 break-all font-mono text-[11px] text-slate-500">
            {fallback.repository_full_name}
            {headSha ? ` · head ${headSha.slice(0, 12)}` : ""}
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={refreshPending}
          onClick={onRefresh}
        >
          <RefreshCw
            className={refreshPending ? "animate-spin" : undefined}
            aria-hidden
          />
          Rafraîchir
        </Button>
      </div>
      {loading ? (
        <p className="mt-4 text-xs text-slate-500" role="status">
          Chargement de l’observation et des preuves…
        </p>
      ) : error ? (
        <p className="mt-4 text-xs text-rose-700" role="alert">
          {error.message}
        </p>
      ) : (
        <div className="mt-4 space-y-3">
          {fallback.object_kind === "repository" ? (
            <p className="text-xs text-muted-foreground">
              Un dépôt décrit une source mutable. Importez une PR ou un commit
              avec son SHA complet pour créer une preuve vérifiable.
            </p>
          ) : (
            coverageItems.map((coverage) => {
              const evidence = evidences.find(
                (item) =>
                  item.requirement_version_public_id ===
                    coverage.requirement_version_public_id &&
                  item.deliverable_public_id === coverage.deliverable_public_id,
              );
              return (
                <EvidenceRow
                  key={coverage.requirement_version_public_id}
                  coverage={coverage}
                  evidence={evidence}
                  createPending={evidencePending}
                  reviewPending={reviewPending}
                  onCreate={() => onCreateEvidence(coverage)}
                  onReview={(decision) =>
                    evidence && onReview(evidence.public_id, decision)
                  }
                />
              );
            })
          )}
          {!coverageItems.length ? (
            <p className="text-xs text-slate-500">
              Générez d’abord un plan avec des exigences et sections durables.
            </p>
          ) : null}
        </div>
      )}
    </article>
  );
}

function EvidenceRow({
  coverage,
  evidence,
  createPending,
  reviewPending,
  onCreate,
  onReview,
}: {
  coverage: CoverageItem;
  evidence?: ExternalEvidence;
  createPending: boolean;
  reviewPending: boolean;
  onCreate: () => void;
  onReview: (decision: "validate" | "reject") => void;
}) {
  return (
    <div className="border-l-2 border-indigo-200 pl-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="text-xs font-semibold">{coverage.requirement_title}</p>
          <p className="mt-1 font-mono text-[10px] text-slate-500">
            {shortId(coverage.requirement_version_public_id)}
          </p>
        </div>
        {evidence ? <StatusPill status={evidence.status} /> : null}
      </div>
      {!evidence ? (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="mt-3"
          disabled={!coverage.section_public_id || createPending}
          onClick={onCreate}
        >
          <GitPullRequest aria-hidden />
          Créer la preuve candidate
        </Button>
      ) : evidence.status === "candidate" ? (
        <div className="mt-3 flex flex-wrap gap-2">
          <Button
            type="button"
            size="sm"
            disabled={reviewPending}
            onClick={() => onReview("validate")}
          >
            <Check aria-hidden />
            Valider humainement
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={reviewPending}
            onClick={() => onReview("reject")}
          >
            <X aria-hidden />
            Rejeter
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function PersistentMutationError({
  error,
  retry,
}: {
  error: Error;
  retry?: () => void;
}) {
  return (
    <div className="mt-4 border border-rose-200 bg-rose-50 p-3" role="alert">
      <p className="text-sm text-rose-800">{error.message}</p>
      {retry ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="mt-3"
          onClick={retry}
        >
          <RotateCcw aria-hidden />
          Réessayer avec la même commande
        </Button>
      ) : null}
    </div>
  );
}
