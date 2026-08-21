import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Bot,
  Check,
  Database,
  Send,
  ShieldCheck,
  Sparkles,
  User,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { FormEvent } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { api } from "@/api/client";
import type { ProposalView } from "@/api/types";
import { ErrorState, LoadingState } from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { formatDate, humanize, shortId } from "@/lib/format";
import { cn } from "@/lib/utils";

export function SessionPage() {
  const { projectId = "", sessionId = "" } = useParams();
  const queryClient = useQueryClient();
  const [content, setContent] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const session = useQuery({
    queryKey: ["session", sessionId],
    queryFn: () => api.session(sessionId),
    enabled: Boolean(sessionId),
  });
  const pending = useMemo(
    () =>
      session.data?.proposals.filter((item) => item.status === "proposed") ??
      [],
    [session.data],
  );
  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
  };
  const send = useMutation({
    mutationFn: () => api.sendMessage(sessionId, content),
    onSuccess: () => {
      setContent("");
      setSelected([]);
      refresh();
    },
    onError: (error) => toast.error(error.message),
  });
  const decide = useMutation({
    mutationFn: (decision: "confirm" | "reject") =>
      api.decideProposals(sessionId, selected, decision),
    onSuccess: (result, decision) => {
      setSelected([]);
      refresh();
      toast.success(
        decision === "confirm"
          ? `${result.confirmed.length} connaissance(s) confirmée(s)`
          : "Propositions rejetées",
      );
      if (result.insight_ids.length)
        toast.warning("Une contradiction demande votre attention");
    },
    onError: (error) => toast.error(error.message),
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    if (content.trim()) send.mutate();
  }
  function toggle(id: string) {
    setSelected((current) =>
      current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id],
    );
  }

  if (session.isLoading)
    return (
      <LoadingState label="Chargement de la session et de son contexte…" />
    );
  if (session.error)
    return <ErrorState error={session.error} retry={() => session.refetch()} />;
  if (!session.data) return null;
  const data = session.data;
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
          <span className="hidden font-mono text-xs text-slate-400 sm:inline">
            {shortId(data.session.public_id)}
          </span>
        </div>
      </header>
      <div className="grid min-h-0 flex-1 lg:grid-cols-[minmax(0,1fr)_410px]">
        <section className="flex min-h-[65dvh] flex-col border-r border-slate-200">
          <div className="flex-1 space-y-6 overflow-y-auto px-4 py-7 sm:px-8 lg:px-10">
            <div className="mx-auto max-w-3xl space-y-6">
              {data.context_pack ? (
                <div className="flex items-center justify-between border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-800">
                  <span className="flex items-center gap-2 font-semibold">
                    <ShieldCheck className="size-4" />
                    ContextPack transmis · aucune reformulation requise
                  </span>
                  <span className="font-mono">
                    {Array.isArray(data.context_pack.provenance)
                      ? data.context_pack.provenance.length
                      : 0}{" "}
                    sources
                  </span>
                </div>
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
                <div className="flex items-center gap-3 text-sm text-slate-500">
                  <Sparkles className="size-4 animate-pulse text-indigo-500" />
                  L’agent structure sa réponse et ses propositions…
                </div>
              ) : null}
            </div>
          </div>
          <form
            onSubmit={submit}
            className="border-t border-slate-200 bg-white p-4 sm:p-6"
          >
            <div className="mx-auto max-w-3xl">
              <Textarea
                value={content}
                onChange={(event) => setContent(event.target.value)}
                placeholder={
                  data.session.node_key === "product"
                    ? "Décrivez une intention, une règle ou un cas limite…"
                    : "Décidez une approche technique à partir du ContextPack…"
                }
                className="min-h-24 resize-none border-slate-300 bg-slate-50 focus:bg-white"
              />
              <div className="mt-3 flex items-center justify-between gap-3">
                <p className="hidden text-xs text-slate-400 sm:block">
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
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
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
          <p className="mt-2 text-xs leading-5 text-slate-500">
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
            <div className="sticky bottom-0 mt-5 grid grid-cols-2 gap-2 bg-[#f7f8fb] pt-3">
              <Button
                variant="outline"
                disabled={!selected.length || decide.isPending}
                onClick={() => decide.mutate("reject")}
              >
                <X />
                Rejeter
              </Button>
              <Button
                disabled={!selected.length || decide.isPending}
                onClick={() => decide.mutate("confirm")}
              >
                <Check />
                Confirmer ({selected.length})
              </Button>
            </div>
          ) : null}
          <div className="mt-6 border-t border-slate-200 pt-5">
            <p className="flex items-center gap-2 text-xs font-semibold text-slate-700">
              <Database className="size-4 text-indigo-500" />
              Provenance de session
            </p>
            <dl className="mt-3 space-y-2 text-xs">
              <div className="flex justify-between">
                <dt className="text-slate-500">Scope</dt>
                <dd className="font-medium">{data.session.scope_kind}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-500">Messages</dt>
                <dd className="font-medium">{data.messages.length}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-slate-500">Dernière activité</dt>
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
            "mt-2 flex items-center gap-2 text-[10px] text-slate-400",
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
        <p className="mt-3 font-mono text-[10px] text-slate-400">
          {proposal.source_data.source_version_ids.length} source(s) reliée(s)
        </p>
      ) : null}
    </button>
  );
}
