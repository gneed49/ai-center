import { notifyRequestError } from "@/lib/request-error";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  AlertTriangle,
  Bot,
  Check,
  Database,
  Send,
  Sparkles,
  User,
  X,
} from "lucide-react";
import { useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { ApiError, api, createIdempotencyKey } from "@/api/client";
import type { ProposalView } from "@/api/types";
import { ContextPackSummaryCard } from "@/components/app/context-pack-summary";
import { ErrorState, LoadingState, NotFoundState } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { isContextPackCurrent } from "@/lib/context-pack";
import { formatDate, humanize, shortId } from "@/lib/format";
import { cn } from "@/lib/utils";

type SendMessageCommand = Readonly<{
  content: string;
  idempotencyKey: string;
}>;

export function SessionPage() {
  const { projectId = "", sessionId = "" } = useParams();
  const queryClient = useQueryClient();
  const [content, setContent] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const composer = useRef<HTMLTextAreaElement>(null);
  const session = useQuery({
    queryKey: ["session", projectId, sessionId],
    queryFn: () => api.session(projectId, sessionId),
    enabled: Boolean(projectId && sessionId),
  });
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
    enabled: Boolean(projectId),
  });
  const pending = useMemo(
    () =>
      session.data?.proposals.filter((item) => item.status === "proposed") ??
      [],
    [session.data],
  );
  const refresh = () => {
    queryClient.invalidateQueries({
      queryKey: ["session", projectId, sessionId],
    });
    queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
  };
  const send = useMutation({
    mutationFn: (command: SendMessageCommand) =>
      api.sendMessage(
        projectId,
        sessionId,
        command.content,
        command.idempotencyKey,
      ),
    onSuccess: () => {
      setContent("");
      setSelected([]);
      refresh();
      requestAnimationFrame(() => composer.current?.focus());
    },
    onError: notifyRequestError,
  });
  const decide = useMutation({
    mutationFn: (command: {
      decision: "confirm" | "reject";
      proposalIds: string[];
      idempotencyKey: string;
    }) =>
      api.decideProposals(
        projectId,
        sessionId,
        command.proposalIds,
        command.decision,
        command.idempotencyKey,
      ),
    onSuccess: (result, command) => {
      setSelected([]);
      refresh();
      toast.success(
        command.decision === "confirm"
          ? `${result.confirmed.length} connaissance(s) confirmée(s)`
          : "Propositions rejetées",
      );
      if (result.insight_ids.length)
        toast.warning("Une contradiction demande votre attention");
    },
    onError: notifyRequestError,
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!content.trim() || send.isPending) return;
    send.mutate({
      content,
      idempotencyKey: createIdempotencyKey(),
    });
  }
  function toggle(id: string) {
    if (decide.isError) decide.reset();
    setSelected((current) =>
      current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id],
    );
  }
  function decideSelected(decision: "confirm" | "reject") {
    const previous = decide.variables;
    const sameCommand =
      decide.isError &&
      previous?.decision === decision &&
      previous.proposalIds.length === selected.length &&
      previous.proposalIds.every((id) => selected.includes(id));
    decide.mutate(
      sameCommand
        ? previous
        : {
            decision,
            proposalIds: [...selected],
            idempotencyKey: createIdempotencyKey(),
          },
    );
  }

  if (session.isLoading || snapshot.isLoading)
    return (
      <LoadingState label="Chargement de la session et de son contexte…" />
    );
  if (session.error instanceof ApiError && session.error.status === 404)
    return (
      <NotFoundState
        title="Session introuvable dans ce projet"
        description="L’identifiant de session ne correspond pas au projet ouvert. Aucun contexte d’un autre projet n’a été chargé."
      />
    );
  if (session.error)
    return <ErrorState error={session.error} retry={() => session.refetch()} />;
  if (snapshot.error)
    return (
      <ErrorState error={snapshot.error} retry={() => snapshot.refetch()} />
    );
  if (!session.data) return null;
  const data = session.data;
  const graphVersion =
    snapshot.data?.project.graph_version ??
    data.context_pack?.source_graph_version ??
    0;
  const techSessionBlocked =
    data.session.node_key === "tech" &&
    !isContextPackCurrent(data.context_pack, graphVersion);

  if (techSessionBlocked) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to={`/projects/${projectId}`}>
            <ArrowLeft />
            Retour au projet
          </Link>
        </Button>
        <div
          className="border border-amber-200 bg-amber-50 p-6 text-amber-950"
          role="alert"
        >
          <div className="flex items-start gap-3">
            <AlertTriangle className="mt-0.5 size-5 shrink-0" />
            <div>
              <h1 className="text-xl font-semibold">Session Tech bloquée</h1>
              <p className="mt-2 max-w-2xl text-sm leading-6 text-amber-800">
                Une session Tech ne peut travailler qu’avec un ContextPack
                courant. Compilez une nouvelle version depuis le handoff avant
                de reprendre cette session.
              </p>
              <Button asChild className="mt-5">
                <Link to={`/projects/${projectId}/handoff`}>
                  Recompiler le ContextPack
                </Link>
              </Button>
            </div>
          </div>
        </div>
        {data.context_pack ? (
          <ContextPackSummaryCard
            pack={data.context_pack}
            graphVersion={graphVersion}
          />
        ) : null}
      </div>
    );
  }
  return (
    <div className="-mx-4 -my-7 flex min-h-[calc(100dvh-4rem)] flex-col bg-white sm:-mx-7 lg:-mx-10 lg:-my-10 lg:min-h-dvh">
      <header className="flex flex-wrap items-center justify-between gap-4 border-b border-slate-200 px-4 py-4 sm:px-7 lg:px-8">
        <div className="flex min-w-0 items-center gap-4">
          <Button variant="ghost" size="icon" asChild>
            <Link to={`/projects/${projectId}`} aria-label="Retour au projet">
              <ArrowLeft />
            </Link>
          </Button>
          <span
            className={cn(
              "grid size-10 shrink-0 place-items-center",
              data.session.node_key === "product"
                ? "bg-indigo-100 text-indigo-700"
                : "bg-emerald-100 text-emerald-700",
            )}
          >
            <Bot className="size-5" />
          </span>
          <div className="min-w-0">
            <p className="truncate font-semibold">{data.session.title}</p>
            <p className="text-xs text-slate-500">
              Agent {data.session.scope_kind} · contexte isolé
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <StatusPill status={data.session.status} />
          <span className="hidden font-mono text-xs text-slate-500 sm:inline">
            {shortId(data.session.public_id)}
          </span>
        </div>
      </header>
      <div className="grid min-h-0 flex-1 lg:grid-cols-[minmax(0,1fr)_410px]">
        <section className="flex min-h-[65dvh] flex-col border-r border-slate-200">
          <div className="flex-1 space-y-6 overflow-y-auto px-4 py-7 sm:px-8 lg:px-10">
            <div className="mx-auto max-w-3xl space-y-6">
              {data.context_pack ? (
                <ContextPackSummaryCard
                  pack={data.context_pack}
                  graphVersion={graphVersion}
                  compact
                />
              ) : null}
              {data.messages.length === 0 ? (
                <div className="border border-dashed border-slate-300 bg-slate-50 p-7">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
                    Session prête
                  </p>
                  <h2 className="mt-2 text-xl font-semibold">
                    Cadrez une intention, pas un document.
                  </h2>
                  <p className="mt-2 text-sm leading-6 text-slate-600">
                    L’agent répond dans son scope, cite le contexte utilisé et
                    soumet chaque mutation à votre validation.
                  </p>
                </div>
              ) : (
                data.messages.map((message) => (
                  <Message
                    key={message.public_id}
                    role={message.role}
                    content={message.content}
                    date={message.created_at}
                    sources={message.metadata.sources}
                  />
                ))
              )}
              {send.isPending ? (
                <div
                  className="flex items-center gap-3 text-sm text-slate-500"
                  role="status"
                  aria-live="polite"
                >
                  <Sparkles className="size-4 animate-pulse text-indigo-500" />
                  L’agent structure sa réponse et ses propositions…
                </div>
              ) : null}
            </div>
          </div>
          <form
            onSubmit={submit}
            className="border-t border-slate-200 bg-white p-4 sm:p-6"
            aria-busy={send.isPending}
          >
            <div className="mx-auto max-w-3xl">
              <label htmlFor="session-message" className="sr-only">
                Message à l’agent {data.session.node_key}
              </label>
              <Textarea
                ref={composer}
                id="session-message"
                value={content}
                onChange={(event) => {
                  if (send.isError) send.reset();
                  setContent(event.target.value);
                }}
                placeholder={
                  data.session.node_key === "product"
                    ? "Décrivez une intention, une règle ou un cas limite…"
                    : "Décidez une approche technique à partir du ContextPack…"
                }
                className="min-h-24 resize-none border-slate-300 bg-slate-50 focus:bg-white"
                aria-describedby="session-message-help"
                disabled={send.isPending}
              />
              {send.error ? (
                <div className="mt-3">
                  <ErrorState
                    error={send.error}
                    title="Le message n’a pas été envoyé"
                    retry={() => {
                      if (send.variables) send.mutate(send.variables);
                    }}
                  />
                </div>
              ) : null}
              <div className="mt-3 flex items-center justify-between gap-3">
                <p
                  id="session-message-help"
                  className="hidden text-xs text-slate-500 sm:block"
                >
                  Les réponses ne modifient jamais le graphe sans confirmation.
                </p>
                <Button
                  type="submit"
                  disabled={!content.trim() || send.isPending}
                >
                  Envoyer
                  <Send />
                </Button>
              </div>
            </div>
          </form>
        </section>
        <aside className="bg-[#f7f8fb] p-4 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-600">
                Review queue
              </p>
              <h2 className="mt-1 text-lg font-semibold">
                Mutations proposées
              </h2>
            </div>
            <span className="grid size-8 place-items-center bg-white text-sm font-bold text-indigo-600 shadow-sm">
              {pending.length}
            </span>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-600">
            Sélectionnez les unités exactes qui deviendront la vérité du projet.
          </p>
          <div className="mt-5 space-y-3">
            {pending.length ? (
              pending.map((proposal) => (
                <Proposal
                  key={proposal.public_id}
                  proposal={proposal}
                  selected={selected.includes(proposal.public_id)}
                  onToggle={() => toggle(proposal.public_id)}
                />
              ))
            ) : (
              <div className="border border-dashed border-slate-300 bg-white p-5 text-center text-sm text-slate-500">
                Aucune proposition en attente.
              </div>
            )}
          </div>
          {pending.length ? (
            <div className="sticky bottom-0 mt-5 space-y-3 bg-[#f7f8fb] pt-3">
              {decide.error ? (
                <ErrorState
                  error={decide.error}
                  title="La décision n’a pas été enregistrée"
                  retry={() => {
                    if (decide.variables) decide.mutate(decide.variables);
                  }}
                />
              ) : null}
              <div className="grid grid-cols-2 gap-2">
                <Button
                  variant="outline"
                  disabled={!selected.length || decide.isPending}
                  onClick={() => decideSelected("reject")}
                  aria-busy={decide.isPending}
                >
                  <X />
                  Rejeter
                </Button>
                <Button
                  disabled={!selected.length || decide.isPending}
                  onClick={() => decideSelected("confirm")}
                  aria-busy={decide.isPending}
                >
                  <Check />
                  Confirmer ({selected.length})
                </Button>
              </div>
            </div>
          ) : null}
          <div className="mt-6 border-t border-slate-200 pt-5">
            <p className="flex items-center gap-2 text-xs font-semibold text-slate-700">
              <Database className="size-4 text-indigo-500" />
              Provenance de session
            </p>
            <dl className="mt-3 space-y-2 text-xs">
              <div className="flex justify-between">
                <dt className="text-slate-600">Scope</dt>
                <dd className="font-medium">{data.session.scope_kind}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-600">Messages</dt>
                <dd className="font-medium">{data.messages.length}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-600">Dernière activité</dt>
                <dd className="font-medium">
                  {formatDate(data.session.updated_at, true)}
                </dd>
              </div>
            </dl>
          </div>
        </aside>
      </div>
    </div>
  );
}

function Message({
  role,
  content,
  date,
  sources,
}: {
  role: string;
  content: string;
  date: string;
  sources?: string[];
}) {
  const user = role === "user";
  return (
    <article className={cn("flex gap-3", user && "flex-row-reverse")}>
      <span
        className={cn(
          "grid size-8 shrink-0 place-items-center",
          user ? "bg-slate-900 text-white" : "bg-indigo-100 text-indigo-700",
        )}
      >
        {user ? <User className="size-4" /> : <Bot className="size-4" />}
      </span>
      <div className={cn("max-w-[82%]", user && "text-right")}>
        <div
          className={cn(
            "border px-4 py-3 text-left text-sm leading-6",
            user
              ? "border-slate-900 bg-slate-900 text-white"
              : "border-slate-200 bg-white text-slate-700",
          )}
        >
          {content}
        </div>
        <div
          className={cn(
            "mt-2 flex items-center gap-2 text-[10px] text-slate-500",
            user && "justify-end",
          )}
        >
          {formatDate(date, true)}
          {sources?.length ? <span>· {sources.length} sources</span> : null}
        </div>
      </div>
    </article>
  );
}

function Proposal({
  proposal,
  selected,
  onToggle,
}: {
  proposal: ProposalView;
  selected: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-pressed={selected}
      aria-label={`${selected ? "Désélectionner" : "Sélectionner"} la proposition ${proposal.title}`}
      className={cn(
        "w-full border bg-white p-4 text-left transition",
        selected
          ? "border-indigo-400 shadow-[inset_3px_0_0_#6366f1]"
          : "border-slate-200 hover:border-slate-300",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-indigo-600">
          {humanize(proposal.entry_type)}
        </span>
        <span
          className={cn(
            "grid size-5 place-items-center border",
            selected
              ? "border-indigo-600 bg-indigo-600 text-white"
              : "border-slate-300 text-transparent",
          )}
        >
          <Check className="size-3" />
        </span>
      </div>
      <p className="mt-3 text-sm font-semibold text-slate-900">
        {proposal.title}
      </p>
      <p className="mt-1 text-sm leading-5 text-slate-600">
        {proposal.statement}
      </p>
      {proposal.source_data.source_version_ids?.length ? (
        <p className="mt-3 font-mono text-[10px] text-slate-500">
          {proposal.source_data.source_version_ids.length} source(s) reliée(s)
        </p>
      ) : null}
    </button>
  );
}
