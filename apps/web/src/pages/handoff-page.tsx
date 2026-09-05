import { notifyRequestError } from "@/lib/request-error";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  FileCheck2,
  PackageOpen,
  Send,
  ShieldCheck,
} from "lucide-react";
import { useRef } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import {
  api,
  createIdempotencyKey,
  createHandoffIdempotencyKeys,
  type HandoffIdempotencyKeys,
} from "@/api/client";
import { ContextPackSummaryCard } from "@/components/app/context-pack-summary";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PageHeader,
} from "@/components/app/page";
import { StatusPill } from "@/components/app/status-pill";
import { Button } from "@/components/ui/button";
import { isContextPackCurrent } from "@/lib/context-pack";
import { shortId } from "@/lib/format";

export function HandoffPage() {
  const { projectId = "" } = useParams();
  const queryClient = useQueryClient();
  const handoffIdempotencyKeys = useRef<HandoffIdempotencyKeys | null>(null);
  const gateIdempotencyKey = useRef<string | null>(null);
  const snapshot = useQuery({
    queryKey: ["snapshot", projectId],
    queryFn: () => api.snapshot(projectId),
  });
  const latestHandoff = useQuery({
    queryKey: ["handoff", projectId, "latest"],
    queryFn: () => api.latestHandoff(projectId),
    enabled: Boolean(projectId),
  });
  const gate = useMutation({
    mutationFn: () => {
      gateIdempotencyKey.current ??= createIdempotencyKey();
      return api.evaluateGate(projectId, gateIdempotencyKey.current);
    },
    onSuccess: () => {
      gateIdempotencyKey.current = null;
      queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
    },
    onError: notifyRequestError,
  });
  const handoff = useMutation({
    mutationFn: async () => {
      const source = snapshot.data?.sessions.find(
        (item) => item.node_key === "product",
      );
      if (!source) throw new Error("Ouvrez d’abord une session Produit.");
      handoffIdempotencyKeys.current ??= createHandoffIdempotencyKeys();
      return api.prepareHandoff(
        projectId,
        source.public_id,
        handoffIdempotencyKeys.current,
      );
    },
    onSuccess: (data) => {
      handoffIdempotencyKeys.current = null;
      queryClient.setQueryData(["handoff", projectId, "latest"], data);
      queryClient.invalidateQueries({ queryKey: ["snapshot", projectId] });
      toast.success("ContextPack transmis à l’agent Tech");
    },
    onError: notifyRequestError,
  });
  if (snapshot.isLoading || latestHandoff.isLoading) return <LoadingState />;
  if (snapshot.error)
    return (
      <ErrorState error={snapshot.error} retry={() => snapshot.refetch()} />
    );
  if (latestHandoff.error)
    return (
      <ErrorState
        error={latestHandoff.error}
        title="Le dernier handoff n’a pas pu être restauré"
        retry={() => latestHandoff.refetch()}
      />
    );
  if (!snapshot.data) return null;
  const data = snapshot.data;
  const result = latestHandoff.data ?? null;
  const productSession = data.sessions.find(
    (item) => item.node_key === "product",
  );
  const gateCurrent = data.gate?.graph_version === data.project.graph_version;
  const ready = Boolean(
    gateCurrent &&
    data.gate &&
    ["passed", "passed_with_warning"].includes(data.gate.status),
  );
  const resultCurrent = isContextPackCurrent(
    result?.context_pack,
    data.project.graph_version,
  );
  const productKnowledge = data.knowledge.filter(
    (item) => item.node_key === "product",
  );
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Transmission contextuelle"
        title="Handoff Produit → Tech"
        description="Compilez une projection immuable et sourcée. L’agent Tech reçoit le nécessaire, pas tout le projet."
        actions={
          <Button variant="outline" asChild>
            <Link to={`/projects/${projectId}`}>
              <ArrowLeft />
              Projet
            </Link>
          </Button>
        }
      />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_390px]">
        <section className="border border-slate-200 bg-white p-5 sm:p-7">
          <div className="grid gap-4 sm:grid-cols-[1fr_88px_1fr] sm:items-center">
            <Scope
              title="Produit"
              subtitle="Source confirmée"
              tone="indigo"
              count={productKnowledge.length}
            />
            <div className="relative flex h-16 items-center justify-center">
              <span className="absolute h-px w-full bg-slate-200" />
              <span className="relative grid size-9 place-items-center border border-slate-200 bg-white">
                <ArrowRight className="size-4 text-indigo-600" />
              </span>
            </div>
            <Scope
              title="Tech"
              subtitle="Session ciblée"
              tone="emerald"
              count={
                result
                  ? 1
                  : data.sessions.filter((item) => item.node_key === "tech")
                      .length
              }
            />
          </div>
          <div className="mt-8 border-t border-slate-200 pt-7">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-slate-500">
                  Préflight
                </p>
                <h2 className="mt-1 text-xl font-semibold">
                  Ce qui sera transmis
                </h2>
              </div>
              {data.gate && gateCurrent ? (
                <StatusPill status={data.gate.status} />
              ) : data.gate ? (
                <StatusPill status="stale" label="À réévaluer" />
              ) : (
                <StatusPill status="pending" />
              )}
            </div>
            <div className="mt-6 grid gap-px border border-slate-200 bg-slate-200 sm:grid-cols-2">
              <PackItem
                icon={FileCheck2}
                label="Objectif et décisions"
                value={
                  1 +
                  productKnowledge.filter((item) =>
                    ["decision", "business_rule"].includes(item.entry_type),
                  ).length
                }
              />
              <PackItem
                icon={ShieldCheck}
                label="Exigences et critères"
                value={
                  productKnowledge.filter((item) =>
                    ["requirement", "acceptance_criterion"].includes(
                      item.entry_type,
                    ),
                  ).length
                }
              />
              <PackItem
                icon={PackageOpen}
                label="Provenance exacte"
                value={productKnowledge.length}
              />
              <PackItem icon={Check} label="Contrat de sortie" value={1} />
            </div>
            {result && resultCurrent && ready ? (
              <div className="mt-6 border border-emerald-200 bg-emerald-50 p-5">
                <div className="flex items-center gap-2 font-semibold text-emerald-900">
                  <Check className="size-4" />
                  Handoff terminé et courant
                </div>
                <p className="mt-2 text-sm text-emerald-800">
                  ContextPack {shortId(result.context_pack_public_id)} compilé
                  et session Tech ouverte. Cet état a été restauré depuis le
                  serveur.
                </p>
                <Button asChild className="mt-4">
                  <Link
                    to={`/projects/${projectId}/sessions/${result.target_session_public_id}`}
                  >
                    Ouvrir la session Tech
                    <ArrowRight />
                  </Link>
                </Button>
              </div>
            ) : !productSession ? (
              <div className="mt-6">
                <EmptyState
                  title="Session Produit requise"
                  description="Le handoff doit conserver la session d’origine dans sa provenance."
                  action={
                    <Button asChild>
                      <Link to={`/projects/${projectId}`}>
                        Démarrer une session
                      </Link>
                    </Button>
                  }
                />
              </div>
            ) : !ready ? (
              <div className="mt-6 border border-amber-200 bg-amber-50 p-5">
                <p className="font-semibold text-amber-950">
                  Le gate doit être évalué.
                </p>
                <p className="mt-1 text-sm text-amber-800">
                  Les éléments manquants seront listés avant toute transmission.
                </p>
                <Button
                  className="mt-4"
                  variant="outline"
                  onClick={() => gate.mutate()}
                  disabled={gate.isPending}
                >
                  {gate.isPending ? "Évaluation…" : "Évaluer le gate"}
                </Button>
              </div>
            ) : (
              <>
                <Button
                  className="mt-6 w-full"
                  size="lg"
                  onClick={() => handoff.mutate()}
                  disabled={handoff.isPending}
                >
                  <Send />
                  {handoff.isPending
                    ? "Compilation du ContextPack…"
                    : result
                      ? "Recompiler et ouvrir une session Tech"
                      : "Compiler et ouvrir la session Tech"}
                </Button>
                {handoff.error ? (
                  <div className="mt-4">
                    <ErrorState
                      error={handoff.error}
                      title="Le handoff n’a pas abouti"
                      retry={() => handoff.mutate()}
                    />
                  </div>
                ) : null}
              </>
            )}
          </div>
        </section>
        <aside className="border border-slate-200 bg-[#11182b] p-5 text-white sm:p-6">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-300">
            ContextPack Preview
          </p>
          <h2 className="mt-2 text-lg font-semibold">Contrat Tech</h2>
          <div className="mt-6 space-y-5">
            <PreviewSection
              label="Objectif"
              value={data.project.objective || "À préciser"}
            />
            <PreviewSection
              label="Règles métier"
              value={`${productKnowledge.filter((item) => item.entry_type === "business_rule").length} version(s)`}
            />
            <PreviewSection
              label="Exigences"
              value={`${productKnowledge.filter((item) => item.entry_type === "requirement").length} version(s)`}
            />
            <PreviewSection
              label="Sortie attendue"
              value="Technical Delivery Plan"
            />
          </div>
          <p className="mt-8 border-t border-white/10 pt-5 text-xs leading-5 text-slate-400">
            Chaque élément transmis référence l’identifiant public et le numéro
            de version exacts. Une mutation ultérieure rendra ce pack obsolète.
          </p>
        </aside>
      </div>
      {result ? (
        <ContextPackSummaryCard
          pack={result.context_pack}
          graphVersion={data.project.graph_version}
        />
      ) : null}
    </div>
  );
}

function Scope({
  title,
  subtitle,
  tone,
  count,
}: {
  title: string;
  subtitle: string;
  tone: "indigo" | "emerald";
  count: number;
}) {
  return (
    <div
      className={
        tone === "indigo"
          ? "border border-indigo-200 bg-indigo-50 p-5"
          : "border border-emerald-200 bg-emerald-50 p-5"
      }
    >
      <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-500">
        {subtitle}
      </p>
      <div className="mt-2 flex items-end justify-between">
        <h3 className="text-xl font-semibold">{title}</h3>
        <strong
          className={
            tone === "indigo"
              ? "text-2xl text-indigo-600"
              : "text-2xl text-emerald-600"
          }
        >
          {count}
        </strong>
      </div>
    </div>
  );
}
function PackItem({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof Check;
  label: string;
  value: number;
}) {
  return (
    <div className="flex items-center justify-between bg-white p-4">
      <span className="flex items-center gap-2 text-sm text-slate-600">
        <Icon className="size-4 text-indigo-500" />
        {label}
      </span>
      <strong>{value}</strong>
    </div>
  );
}
function PreviewSection({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-500">
        {label}
      </p>
      <p className="mt-1 text-sm leading-6 text-slate-200">{value}</p>
    </div>
  );
}
