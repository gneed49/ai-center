import { notifyRequestError } from "@/lib/request-error";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  FilePenLine,
  GitCompareArrows,
  Link2,
  ShieldAlert,
  X,
} from "lucide-react";
import { useRef, useState } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { ApiError, api, createIdempotencyKey } from "@/api/client";
import type { ResolveInsightInput } from "@/api/types";
import {
  ErrorState,
  LoadingState,
  NotFoundState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { formatDate, humanize, shortId } from "@/lib/format";

type ResolutionCommand = {
  projectId: string;
  input: ResolveInsightInput;
  idempotencyKey: string;
};

export function InsightDetailPage() {
  const { projectId = "", insightId = "" } = useParams();
  const queryClient = useQueryClient();
  const [justification, setJustification] = useState("");
  const [resolutionOpen, setResolutionOpen] = useState(false);
  const [selectedSourceId, setSelectedSourceId] = useState("");
  const [revisedStatement, setRevisedStatement] = useState("");
  const [rationale, setRationale] = useState("");
  const resolutionIdempotencyKey = useRef<string | null>(null);
  const [resolutionNeedsRefresh, setResolutionNeedsRefresh] = useState(false);
  const [resolutionNotice, setResolutionNotice] = useState<string | null>(null);
  const [refreshingResolution, setRefreshingResolution] = useState(false);
  const insight = useQuery({
    queryKey: ["insight", projectId, insightId],
    queryFn: () => api.insight(projectId, insightId),
  });
  const action = useMutation({
    mutationFn: (command: {
      decision: "accept" | "dismiss";
      justification: string;
      idempotencyKey: string;
    }) => {
      const ownerProjectId =
        projectId || insight.data?.insight.project_public_id;
      if (!ownerProjectId) throw new Error("Projet du signal introuvable.");
      return api.actOnInsight(
        ownerProjectId,
        insightId,
        command.decision,
        command.justification,
        command.idempotencyKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["insight", projectId, insightId],
      });
      queryClient.invalidateQueries({ queryKey: ["insights"] });
      setJustification("");
      toast.success("Décision auditée");
    },
    onError: notifyRequestError,
  });
  const act = (decision: "accept" | "dismiss") => {
    const previous = action.variables;
    const sameCommand =
      action.isError &&
      previous?.decision === decision &&
      previous.justification === justification;
    action.mutate(
      sameCommand && previous
        ? previous
        : {
            decision,
            justification,
            idempotencyKey: createIdempotencyKey(),
          },
    );
  };
  const resolve = useMutation({
    mutationFn: (command: ResolutionCommand) =>
      api.resolveInsight(
        command.projectId,
        insightId,
        command.input,
        command.idempotencyKey,
      ),
    onSuccess: () => {
      const ownerProjectId =
        projectId || insight.data?.insight.project_public_id;
      queryClient.invalidateQueries({
        queryKey: ["insight", projectId, insightId],
      });
      queryClient.invalidateQueries({ queryKey: ["insights"] });
      if (ownerProjectId) {
        queryClient.invalidateQueries({
          queryKey: ["snapshot", ownerProjectId],
        });
        queryClient.invalidateQueries({
          queryKey: ["handoff", ownerProjectId],
        });
      }
      setResolutionOpen(false);
      resolutionIdempotencyKey.current = null;
      setJustification("");
      setRevisedStatement("");
      setRationale("");
      toast.success("Contexte révisé et signal réévalué");
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 409) {
        setResolutionNeedsRefresh(true);
        setResolutionNotice(
          "Le contexte a changé. Rechargez les sources avant de soumettre votre révision conservée.",
        );
      }
      notifyRequestError(error);
    },
  });
  if (insight.isLoading) return <LoadingState />;
  if (insight.error instanceof ApiError && insight.error.status === 404)
    return <NotFoundState title="Signal introuvable dans ce projet" />;
  if (insight.error)
    return <ErrorState error={insight.error} retry={() => insight.refetch()} />;
  if (!insight.data) return null;
  const data = insight.data;
  const ownerProjectId = projectId || data.insight.project_public_id;
  const open = ["candidate", "open", "accepted"].includes(data.insight.status);
  const resolvableSources = data.sources.filter(
    (source) => source.knowledge_public_id && source.version_public_id,
  );
  function submitResolution() {
    if (resolve.isPending || resolutionNeedsRefresh || refreshingResolution)
      return;
    const source = data.sources.find(
      (item) => item.knowledge_public_id === selectedSourceId,
    );
    if (!source?.knowledge_public_id || !source.version_public_id) return;
    resolutionIdempotencyKey.current ??= createIdempotencyKey();
    resolve.mutate({
      projectId: ownerProjectId,
      idempotencyKey: resolutionIdempotencyKey.current,
      input: {
        expected_graph_version: data.insight.project_graph_version,
        justification,
        mutations: [
          {
            kind: "revise_knowledge",
            knowledge_public_id: source.knowledge_public_id,
            expected_version_public_id: source.version_public_id,
            statement: revisedStatement,
            rationale: rationale || undefined,
          },
        ],
      },
    });
  }
  async function refreshResolution() {
    setRefreshingResolution(true);
    try {
      const [updated, snapshot] = await Promise.all([
        insight.refetch(),
        api.snapshot(ownerProjectId),
      ]);
      if (updated.error) throw updated.error;
      const source = updated.data?.sources.find(
        (item) => item.knowledge_public_id === selectedSourceId,
      );
      const currentSource = snapshot.knowledge.find(
        (item) => item.public_id === selectedSourceId,
      );
      if (
        !source ||
        source.version_public_id !== currentSource?.version_public_id
      ) {
        setResolutionNotice(
          "Cette source a déjà été révisée. Votre saisie est conservée ; consultez les nouveaux signaux avant de décider.",
        );
        return;
      }
      resolutionIdempotencyKey.current = null;
      resolve.reset();
      setResolutionNeedsRefresh(false);
      setResolutionNotice(
        "Contexte actualisé. Vérifiez les sources et votre révision conservée avant de confirmer.",
      );
    } catch (error) {
      if (error instanceof Error) notifyRequestError(error);
    } finally {
      setRefreshingResolution(false);
    }
  }
  function openResolution() {
    const source = resolvableSources[0];
    if (!source?.knowledge_public_id) return;
    setSelectedSourceId(source.knowledge_public_id);
    setRevisedStatement(source.version_statement ?? "");
    setResolutionOpen(true);
  }
  function changeResolutionSource(knowledgeId: string) {
    const source = resolvableSources.find(
      (item) => item.knowledge_public_id === knowledgeId,
    );
    setSelectedSourceId(knowledgeId);
    setRevisedStatement(source?.version_statement ?? "");
    if (resolve.isError) {
      resolve.reset();
      resolutionIdempotencyKey.current = null;
    }
  }
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow={`Signal steward · ${data.insight.project_name}`}
        title={data.insight.title}
        description={data.insight.explanation}
        actions={
          <>
            <Button variant="outline" asChild>
              <Link to="/insights">
                <ArrowLeft />
                Decision Inbox
              </Link>
            </Button>
            <Button variant="ghost" asChild>
              <Link to={`/projects/${ownerProjectId}`}>Projet</Link>
            </Button>
          </>
        }
      />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_390px]">
        <div className="space-y-6">
          <section
            className={
              data.insight.severity === "blocking"
                ? "border border-red-200 bg-red-50 p-5 sm:p-7"
                : "border border-amber-200 bg-amber-50 p-5 sm:p-7"
            }
          >
            <div className="flex flex-wrap items-center gap-2">
              <StatusPill status={data.insight.severity} />
              <StatusPill status={data.insight.status} />
            </div>
            <div className="mt-6 grid gap-5 sm:grid-cols-3">
              <Fact
                label="Confiance"
                value={`${Math.round(data.insight.confidence * 100)}%`}
              />
              <Fact
                label="Détecté"
                value={formatDate(data.insight.detected_at, true)}
              />
              <Fact label="Type" value={humanize(data.insight.insight_type)} />
            </div>
            {data.insight.resolution_justification ? (
              <div className="mt-6 border-t border-current/10 pt-5">
                <p className="text-xs font-semibold uppercase tracking-[0.14em]">
                  Justification
                </p>
                <p className="mt-2 text-sm leading-6">
                  {data.insight.resolution_justification}
                </p>
              </div>
            ) : null}
          </section>
          <section className="border border-slate-200 bg-white">
            <div className="flex items-center gap-3 border-b border-slate-200 px-5 py-4 sm:px-6">
              <GitCompareArrows className="size-5 text-indigo-500" />
              <div>
                <p className="font-semibold">Sources mises en regard</p>
                <p className="text-xs text-slate-500">
                  Versions exactes, jamais les résumés courants
                </p>
              </div>
            </div>
            <div className="grid gap-px bg-slate-200 md:grid-cols-2">
              {data.sources.map((source) => (
                <article
                  key={`${source.source_role}-${source.object_public_id}`}
                  className="bg-white p-5 sm:p-6"
                >
                  <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-indigo-600">
                    {humanize(source.source_role)}
                  </p>
                  <h3 className="mt-3 font-semibold">
                    {source.version_title ?? "Source"}
                  </h3>
                  <p className="mt-2 text-sm leading-6 text-slate-600">
                    {source.version_statement}
                  </p>
                  <p className="mt-5 flex items-center gap-2 font-mono text-[10px] text-slate-500">
                    <Link2 className="size-3" />
                    {shortId(source.object_public_id)}
                  </p>
                </article>
              ))}
            </div>
          </section>
        </div>
        <aside className="border border-slate-200 bg-white p-5 sm:p-6">
          <div className="flex items-center gap-2">
            <ShieldAlert className="size-5 text-indigo-500" />
            <h2 className="font-semibold">Décision humaine</h2>
          </div>
          <p className="mt-2 text-sm leading-6 text-slate-600">
            Acceptez le risque, écartez un faux positif, ou révisez une source
            pour résoudre réellement le signal. Chaque décision est auditée.
          </p>
          {open ? (
            <>
              <Textarea
                id="insight-justification"
                value={justification}
                disabled={
                  action.isPending || resolve.isPending || refreshingResolution
                }
                onChange={(event) => {
                  if (action.isError) action.reset();
                  if (resolve.isError) {
                    resolve.reset();
                    resolutionIdempotencyKey.current = null;
                  }
                  setJustification(event.target.value);
                }}
                placeholder="Expliquez la décision…"
                className="mt-5 min-h-28"
                aria-label="Justification de la décision"
              />
              <div className="mt-4 grid gap-2">
                <Button
                  disabled={
                    !justification.trim() ||
                    action.isPending ||
                    resolve.isPending ||
                    refreshingResolution
                  }
                  onClick={() => act("accept")}
                >
                  <Check />
                  Accepter le signal
                </Button>
                <Button
                  variant="outline"
                  disabled={
                    !justification.trim() ||
                    action.isPending ||
                    resolve.isPending ||
                    refreshingResolution ||
                    !resolvableSources.length
                  }
                  onClick={openResolution}
                  aria-expanded={resolutionOpen}
                >
                  <FilePenLine />
                  Réviser pour résoudre
                </Button>
                <Button
                  variant="ghost"
                  disabled={
                    !justification.trim() ||
                    action.isPending ||
                    resolve.isPending ||
                    refreshingResolution
                  }
                  onClick={() => act("dismiss")}
                >
                  <X />
                  Rejeter le signal
                </Button>
              </div>
              {action.error ? (
                <div className="mt-4">
                  <ErrorState
                    error={action.error}
                    title="La décision n’a pas été enregistrée"
                    retry={() => {
                      if (action.variables) action.mutate(action.variables);
                    }}
                  />
                </div>
              ) : null}
              {resolutionOpen ? (
                <form
                  className="mt-5 space-y-4 border-t border-slate-200 pt-5"
                  onSubmit={(event) => {
                    event.preventDefault();
                    submitResolution();
                  }}
                >
                  <div>
                    <label
                      htmlFor="resolution-source"
                      className="text-xs font-semibold text-slate-700"
                    >
                      Connaissance à réviser
                    </label>
                    <select
                      id="resolution-source"
                      className="mt-2 h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                      value={selectedSourceId}
                      disabled={resolve.isPending || refreshingResolution}
                      onChange={(event) =>
                        changeResolutionSource(event.target.value)
                      }
                    >
                      {resolvableSources.map((source) => (
                        <option
                          key={source.version_public_id}
                          value={source.knowledge_public_id ?? ""}
                        >
                          {source.version_title ?? "Source versionnée"}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label
                      htmlFor="revised-statement"
                      className="text-xs font-semibold text-slate-700"
                    >
                      Nouvelle vérité du projet
                    </label>
                    <Textarea
                      id="revised-statement"
                      className="mt-2 min-h-32"
                      value={revisedStatement}
                      disabled={resolve.isPending || refreshingResolution}
                      onChange={(event) => {
                        if (resolve.isError) {
                          resolve.reset();
                          resolutionIdempotencyKey.current = null;
                        }
                        setRevisedStatement(event.target.value);
                      }}
                      required
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="resolution-rationale"
                      className="text-xs font-semibold text-slate-700"
                    >
                      Rationale de la révision (optionnel)
                    </label>
                    <Textarea
                      id="resolution-rationale"
                      className="mt-2 min-h-20"
                      value={rationale}
                      disabled={resolve.isPending || refreshingResolution}
                      onChange={(event) => {
                        if (resolve.isError) {
                          resolve.reset();
                          resolutionIdempotencyKey.current = null;
                        }
                        setRationale(event.target.value);
                      }}
                    />
                  </div>
                  {resolutionNotice ? (
                    <div
                      className="border border-amber-200 bg-amber-50 p-4 text-sm text-amber-950"
                      role="status"
                    >
                      <p>{resolutionNotice}</p>
                      {resolutionNeedsRefresh ? (
                        <Button
                          type="button"
                          variant="outline"
                          className="mt-3"
                          disabled={refreshingResolution}
                          onClick={() => void refreshResolution()}
                        >
                          {refreshingResolution
                            ? "Actualisation…"
                            : "Recharger les sources"}
                        </Button>
                      ) : null}
                    </div>
                  ) : null}
                  <div className="grid grid-cols-2 gap-2">
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={resolve.isPending || refreshingResolution}
                      onClick={() => setResolutionOpen(false)}
                    >
                      Annuler
                    </Button>
                    <Button
                      type="submit"
                      disabled={
                        resolve.isPending ||
                        resolutionNeedsRefresh ||
                        refreshingResolution ||
                        !justification.trim() ||
                        !revisedStatement.trim()
                      }
                    >
                      {resolve.isPending
                        ? "Résolution…"
                        : "Réviser et résoudre"}
                    </Button>
                  </div>
                  {resolve.error ? (
                    <ErrorState
                      error={resolve.error}
                      title="La résolution atomique a échoué"
                      retry={
                        resolutionNeedsRefresh
                          ? undefined
                          : () => {
                              if (resolve.variables)
                                resolve.mutate(resolve.variables);
                            }
                      }
                    />
                  ) : null}
                </form>
              ) : null}
            </>
          ) : (
            <div className="mt-5 border border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-800">
              Cette décision est close. Une nouvelle version du graphe pourra
              déclencher une réévaluation.
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] font-semibold uppercase tracking-[0.14em] opacity-60">
        {label}
      </p>
      <p className="mt-1 text-lg font-semibold">{value}</p>
    </div>
  );
}
