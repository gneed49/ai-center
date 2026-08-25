import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowRight,
  BookOpen,
  CheckCircle2,
  GitBranch,
  History,
  Inbox,
  Network,
  PackageCheck,
  Play,
  Radio,
  Send,
} from "lucide-react";
import { useRef } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { toast } from "sonner";

import { api, createIdempotencyKey } from "@/api/client";
import type {
  ContextNode,
  KnowledgeSummary,
  SessionSummary,
} from "@/api/types";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { isContextPackCurrent } from "@/lib/context-pack";
import { formatDate, humanize, shortId } from "@/lib/format";

export function ProjectPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const gateIdempotencyKey = useRef<string | null>(null);
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
    enabled: Boolean(projectId),
  });
  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
  const createSession = useMutation({
    mutationFn: (command: { nodeKey: string; idempotencyKey: string }) =>
      api.createSession(
        projectId,
        command.nodeKey,
        undefined,
        command.idempotencyKey,
      ),
    onSuccess: (session) => {
      refresh();
      navigate(`/projects/${projectId}/sessions/${session.session.public_id}`);
    },
    onError: (error) => toast.error(error.message),
  });
  const gate = useMutation({
    mutationFn: () => {
      gateIdempotencyKey.current ??= createIdempotencyKey();
      return api.evaluateGate(projectId, gateIdempotencyKey.current);
    },
    onSuccess: (result) => {
      gateIdempotencyKey.current = null;
      refresh();
      toast[result.status === "blocked" ? "warning" : "success"](
        result.status === "blocked"
          ? "Gate incomplet"
          : "Produit prêt pour le handoff",
      );
    },
    onError: (error) => toast.error(error.message),
  });
  const startSession = (nodeKey: string) => {
    const previous = createSession.variables;
    createSession.mutate(
      createSession.isError && previous?.nodeKey === nodeKey
        ? previous
        : { nodeKey, idempotencyKey: createIdempotencyKey() },
    );
  };
  if (snapshot.isLoading) return <LoadingState />;
  if (snapshot.error)
    return (
      <ErrorState error={snapshot.error} retry={() => snapshot.refetch()} />
    );
  if (!snapshot.data) return null;
  const data = snapshot.data;
  const openInsights = data.insights.filter((item) =>
    ["open", "candidate", "accepted"].includes(item.status),
  );
  const gateCurrent = data.gate?.graph_version === data.project.graph_version;

  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow={`Projet · graphe v${data.project.graph_version}`}
        title={data.project.name}
        description={
          data.project.objective ||
          "Objectif à préciser dans une session Produit."
        }
        actions={
          <>
            <Button variant="outline" asChild>
              <Link to={`/projects/${projectId}/history`}>
                <History />
                Historique
              </Link>
            </Button>
            <Button asChild>
              <Link to={`/projects/${projectId}/handoff`}>
                <Send />
                Préparer le handoff
              </Link>
            </Button>
          </>
        }
      />
      <section className="grid gap-px border border-slate-200 bg-slate-200 sm:grid-cols-2 xl:grid-cols-4">
        <ProjectMetric
          icon={BookOpen}
          label="Connaissances"
          value={data.knowledge.length}
          detail={`${data.knowledge.filter((item) => item.node_key === "product").length} Produit`}
        />
        <ProjectMetric
          icon={PackageCheck}
          label="Livrables"
          value={data.deliverables.length}
          detail={`${data.deliverables.filter((item) => item.status === "committed").length} validés`}
        />
        <ProjectMetric
          icon={Inbox}
          label="Décisions"
          value={openInsights.length}
          detail="à traiter"
          alert={openInsights.some((item) => item.severity === "blocking")}
        />
        <ProjectMetric
          icon={Radio}
          label="Sessions"
          value={data.sessions.length}
          detail={`${data.sessions.filter((item) => item.status === "active").length} actives`}
        />
      </section>
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_380px]">
        <div className="space-y-6">
          <section className="border border-slate-200 bg-white p-5 sm:p-7">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
                  Workflow contextuel
                </p>
                <h2 className="mt-1 text-xl font-semibold tracking-[-0.025em]">
                  Scopes et transmissions
                </h2>
              </div>
              <GitBranch className="size-5 text-indigo-500" />
            </div>
            <div className="mt-7 grid gap-4 min-[1400px]:grid-cols-[minmax(0,1fr)_64px_minmax(0,1fr)] min-[1400px]:items-stretch">
              <NodeCard
                node={data.nodes.find((node) => node.node_key === "product")}
                sessions={data.sessions}
                knowledge={data.knowledge}
                projectId={projectId}
                onStart={() => startSession("product")}
                pending={createSession.isPending}
              />
              <div className="flex items-center justify-center">
                <div className="relative flex h-full min-h-16 w-full items-center justify-center min-[1400px]:min-h-0">
                  <span className="absolute h-px w-full bg-slate-200" />
                  <span className="relative grid size-9 place-items-center border border-slate-200 bg-white">
                    <ArrowRight className="size-4 rotate-90 text-indigo-500 min-[1400px]:rotate-0" />
                  </span>
                </div>
              </div>
              <NodeCard
                node={data.nodes.find((node) => node.node_key === "tech")}
                sessions={data.sessions}
                knowledge={data.knowledge}
                projectId={projectId}
                onStart={() => startSession("tech")}
                pending={createSession.isPending}
              />
            </div>
            {createSession.error ? (
              <div className="mt-5">
                <ErrorState
                  error={createSession.error}
                  title="La session n’a pas été créée"
                  retry={() => {
                    if (createSession.variables)
                      createSession.mutate(createSession.variables);
                  }}
                />
              </div>
            ) : null}
          </section>
          <section className="border border-slate-200 bg-white">
            <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4 sm:px-6">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
                  Graphe confirmé
                </p>
                <h2 className="mt-1 text-lg font-semibold">
                  Connaissances courantes
                </h2>
              </div>
              <span className="font-mono text-xs text-slate-500">
                {data.knowledge.length} entrées
              </span>
            </div>
            {data.knowledge.length ? (
              <div className="divide-y divide-slate-100">
                {data.knowledge
                  .slice()
                  .reverse()
                  .slice(0, 6)
                  .map((item) => (
                    <KnowledgeRow key={item.version_public_id} item={item} />
                  ))}
              </div>
            ) : (
              <div className="p-5">
                <EmptyState
                  title="Le graphe est vide"
                  description="Démarrez la session Produit pour proposer les premières connaissances."
                />
              </div>
            )}
          </section>
        </div>
        <aside className="space-y-6">
          <section className="border border-slate-200 bg-white p-5 sm:p-6">
            <div className="flex items-center justify-between">
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
                ProductReadyGate
              </p>
              {data.gate && gateCurrent ? (
                <StatusPill status={data.gate.status} />
              ) : data.gate ? (
                <StatusPill status="stale" label="À réévaluer" />
              ) : (
                <StatusPill status="pending" />
              )}
            </div>
            <div className="mt-5 grid grid-cols-3 gap-2">
              {[
                ["Règles", data.gate?.counts.business_rules ?? 0],
                ["Exigences", data.gate?.counts.requirements ?? 0],
                ["Critères", data.gate?.counts.acceptance_criteria ?? 0],
              ].map(([label, value]) => (
                <div
                  key={label}
                  className="border border-slate-100 bg-slate-50 p-3"
                >
                  <strong className="block text-xl">{value}</strong>
                  <span className="text-[11px] text-slate-500">{label}</span>
                </div>
              ))}
            </div>
            {data.gate && !gateCurrent ? (
              <p
                className="mt-4 text-sm leading-6 text-amber-700"
                role="status"
              >
                Le gate a été évalué sur le graphe v{data.gate.graph_version}.
                Le projet est maintenant en v{data.project.graph_version}.
              </p>
            ) : data.gate?.missing.length ? (
              <ul className="mt-4 space-y-2 text-xs leading-5 text-red-700">
                {data.gate.missing.map((item) => (
                  <li key={item}>— {item}</li>
                ))}
              </ul>
            ) : (
              <p className="mt-4 text-sm leading-6 text-slate-600">
                Le gate vérifie le minimum Produit et explique chaque manque.
              </p>
            )}
            <Button
              className="mt-5 w-full"
              variant={data.gate?.status === "passed" ? "outline" : "default"}
              onClick={() => gate.mutate()}
              disabled={gate.isPending}
              aria-busy={gate.isPending}
            >
              {gate.isPending ? "Évaluation…" : "Évaluer maintenant"}
            </Button>
            {gate.error ? (
              <div className="mt-4">
                <ErrorState
                  error={gate.error}
                  title="Le gate n’a pas pu être évalué"
                  retry={() => gate.mutate()}
                />
              </div>
            ) : null}
          </section>
          <section className="border border-slate-200 bg-[#11182b] p-5 text-white sm:p-6">
            <div className="flex items-center gap-3">
              <Network className="size-5 text-indigo-300" />
              <h2 className="font-semibold">Continuité vérifiable</h2>
            </div>
            <div className="mt-5 space-y-4 text-xs text-slate-300">
              <FlowLine
                label="Intention"
                value={data.project.objective ? "Présente" : "À cadrer"}
              />
              <FlowLine
                label="Graphe"
                value={`v${data.project.graph_version}`}
              />
              <FlowLine
                label="Dernier ContextPack"
                value={
                  data.latest_handoff
                    ? `v${data.latest_handoff.context_pack.version} · ${
                        isContextPackCurrent(
                          data.latest_handoff.context_pack,
                          data.project.graph_version,
                        )
                          ? "courant"
                          : "obsolète"
                      }`
                    : "Aucun"
                }
              />
              <FlowLine
                label="Projections"
                value={String(data.deliverables.length)}
              />
            </div>
            <Button
              asChild
              className="mt-6 w-full bg-white text-slate-950 hover:bg-slate-100"
            >
              <Link to={`/projects/${projectId}/deliverables`}>
                Voir les livrables
                <ArrowRight />
              </Link>
            </Button>
          </section>
          {openInsights.length ? (
            <section className="border border-red-200 bg-red-50 p-5">
              <div className="flex items-center justify-between">
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-red-700">
                  Signal actif
                </p>
                <StatusPill status={openInsights[0].severity} />
              </div>
              <h3 className="mt-4 font-semibold text-red-950">
                {openInsights[0].title}
              </h3>
              <p className="mt-2 text-sm leading-6 text-red-800">
                {openInsights[0].explanation}
              </p>
              <Button
                variant="outline"
                asChild
                className="mt-4 border-red-300 bg-white"
              >
                <Link
                  to={`/projects/${projectId}/insights/${openInsights[0].public_id}`}
                >
                  Examiner
                  <ArrowRight />
                </Link>
              </Button>
            </section>
          ) : null}
        </aside>
      </div>
    </div>
  );
}

function ProjectMetric({
  icon: Icon,
  label,
  value,
  detail,
  alert,
}: {
  icon: typeof BookOpen;
  label: string;
  value: number;
  detail: string;
  alert?: boolean;
}) {
  return (
    <div className="bg-white p-5">
      <div className="flex items-center justify-between text-xs text-slate-500">
        <span>{label}</span>
        <Icon
          className={alert ? "size-4 text-red-500" : "size-4 text-indigo-500"}
        />
      </div>
      <div className="mt-3 flex items-baseline gap-2">
        <strong className="text-2xl font-semibold">{value}</strong>
        <span className="text-xs text-slate-500">{detail}</span>
      </div>
    </div>
  );
}

function NodeCard({
  node,
  sessions,
  knowledge,
  projectId,
  onStart,
  pending,
}: {
  node?: ContextNode;
  sessions: SessionSummary[];
  knowledge: KnowledgeSummary[];
  projectId: string;
  onStart: () => void;
  pending: boolean;
}) {
  if (!node) return null;
  const latest = sessions.find((session) => session.node_key === node.node_key);
  const count = knowledge.filter(
    (item) => item.node_key === node.node_key,
  ).length;
  return (
    <article className="border border-slate-200 bg-slate-50/60 p-5">
      <div className="flex items-start justify-between gap-3">
        <span
          className={
            node.node_key === "product"
              ? "grid size-10 place-items-center bg-indigo-100 text-indigo-700"
              : "grid size-10 place-items-center bg-emerald-100 text-emerald-700"
          }
        >
          <Network className="size-5" />
        </span>
        <StatusPill
          status={latest?.status ?? "pending"}
          label={latest ? "Session active" : "À ouvrir"}
        />
      </div>
      <p className="mt-5 text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
        {node.profile_name}
      </p>
      <h3 className="mt-1 text-xl font-semibold">{node.title}</h3>
      <p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">
        {node.description}
      </p>
      <div className="mt-5 flex items-center gap-4 border-t border-slate-200 pt-4 text-xs text-slate-500">
        <span>{count} connaissances</span>
        {latest ? <span>maj {formatDate(latest.updated_at)}</span> : null}
      </div>
      {node.node_key === "tech" ? (
        <Button asChild className="mt-4 w-full" variant="outline">
          <Link to={`/projects/${projectId}/handoff`}>
            <Send />
            Préparer via handoff
          </Link>
        </Button>
      ) : (
        <Button className="mt-4 w-full" onClick={onStart} disabled={pending}>
          <Play />
          Nouvelle session
        </Button>
      )}
      {latest ? (
        <Button asChild variant="ghost" className="mt-1 w-full">
          <Link to={`/projects/${projectId}/sessions/${latest.public_id}`}>
            Reprendre la dernière
            <ArrowRight />
          </Link>
        </Button>
      ) : null}
    </article>
  );
}

function KnowledgeRow({ item }: { item: KnowledgeSummary }) {
  return (
    <div className="grid gap-3 px-5 py-4 sm:grid-cols-[130px_1fr_auto] sm:items-start sm:px-6">
      <div>
        <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-indigo-600">
          {humanize(item.entry_type)}
        </span>
        <p className="mt-1 font-mono text-[10px] text-slate-500">
          {shortId(item.version_public_id)} · v{item.version_number}
        </p>
      </div>
      <div>
        <p className="text-sm font-semibold text-slate-900">{item.title}</p>
        <p className="mt-1 text-sm leading-6 text-slate-600">
          {item.statement}
        </p>
      </div>
      <span className="text-xs text-slate-500">{item.node_key}</span>
    </div>
  );
}

function FlowLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between border-b border-white/10 pb-3">
      <span>{label}</span>
      <span className="flex items-center gap-1.5 font-medium text-white">
        <CheckCircle2 className="size-3.5 text-emerald-400" />
        {value}
      </span>
    </div>
  );
}
