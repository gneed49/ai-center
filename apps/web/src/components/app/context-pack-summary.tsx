import {
  CheckCircle2,
  ChevronDown,
  CircleMinus,
  Fingerprint,
  GitBranch,
  PackageCheck,
} from "lucide-react";

import type { ContextPackSelectionItem, ContextPackSummary } from "@/api/types";
import { StatusPill } from "@/components/app/status-pill";
import { isContextPackCurrent } from "@/lib/context-pack";
import { formatDate, humanize, shortId } from "@/lib/format";

export function ContextPackSummaryCard({
  pack,
  graphVersion,
  compact = false,
}: {
  pack: ContextPackSummary;
  graphVersion: number;
  compact?: boolean;
}) {
  const current = isContextPackCurrent(pack, graphVersion);
  const included = (pack.selection_items ?? []).filter(
    (item) => item.decision === "included",
  );
  const excluded = (pack.selection_items ?? []).filter(
    (item) => item.decision === "excluded",
  );

  return (
    <section
      aria-label={`ContextPack version ${pack.version}`}
      className="border border-slate-200 bg-white"
    >
      <div className="flex flex-wrap items-start justify-between gap-4 border-b border-slate-200 p-4 sm:p-5">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <PackageCheck className="size-4 text-indigo-600" />
            <h2 className="font-semibold">ContextPack v{pack.version}</h2>
            <StatusPill
              status={current ? "current" : "stale"}
              label={current ? "Courant" : "Obsolète"}
            />
          </div>
          <p className="mt-2 text-xs text-slate-500">
            Compilé {formatDate(pack.compiled_at, true)} · sélection{" "}
            {humanize(pack.selection_mode)}
          </p>
        </div>
        <span className="font-mono text-xs text-slate-500">
          {shortId(pack.public_id)}
        </span>
      </div>

      {!current ? (
        <div role="alert" className="border-b border-amber-200 bg-amber-50 p-4">
          <p className="text-sm font-semibold text-amber-950">
            Ce pack cible le graphe v{pack.source_graph_version}, alors que le
            projet est en v{graphVersion}.
          </p>
          <p className="mt-1 text-xs leading-5 text-amber-800">
            {pack.stale_reason
              ? `${pack.stale_reason} `
              : "Le contexte source a changé. "}
            Il reste consultable pour l’audit, mais ne peut plus alimenter une
            nouvelle génération.
          </p>
        </div>
      ) : null}

      <dl className="grid gap-px bg-slate-200 sm:grid-cols-2 xl:grid-cols-4">
        <Fact
          icon={GitBranch}
          label="Version du graphe"
          value={`v${pack.source_graph_version}`}
        />
        <Fact
          icon={CheckCircle2}
          label="Sources retenues"
          value={String(included.length)}
        />
        <Fact
          icon={CircleMinus}
          label="Sources écartées"
          value={String(excluded.length)}
        />
        <Fact
          icon={Fingerprint}
          label="Budget contexte"
          value={`${pack.token_count}/${pack.token_budget}`}
        />
      </dl>

      {compact ? null : (
        <div className="p-4 sm:p-5">
          <dl className="grid gap-3 text-xs sm:grid-cols-2">
            <Meta label="Compilateur" value={pack.compiler_version} />
            <Meta label="Empreinte SHA-256" value={pack.content_hash} mono />
          </dl>
          <SelectionList title="Contexte inclus" items={included} pack={pack} />
          {excluded.length ? (
            <SelectionList
              title="Contexte exclu"
              items={excluded}
              pack={pack}
            />
          ) : null}
        </div>
      )}
    </section>
  );
}

function Fact({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof GitBranch;
  label: string;
  value: string;
}) {
  return (
    <div className="bg-white p-4">
      <dt className="flex items-center gap-2 text-[11px] text-slate-500">
        <Icon className="size-3.5 text-indigo-500" />
        {label}
      </dt>
      <dd className="mt-2 font-mono text-sm font-semibold text-slate-900">
        {value}
      </dd>
    </div>
  );
}

function Meta({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="min-w-0 border border-slate-100 bg-slate-50 p-3">
      <dt className="text-slate-500">{label}</dt>
      <dd
        className={
          mono
            ? "mt-1 truncate font-mono text-slate-800"
            : "mt-1 font-medium text-slate-800"
        }
        title={value}
      >
        {value}
      </dd>
    </div>
  );
}

function SelectionList({
  title,
  items,
  pack,
}: {
  title: string;
  items: ContextPackSelectionItem[];
  pack: ContextPackSummary;
}) {
  const includedKnowledge = new Map(
    (pack.content.knowledge ?? []).map((item) => [
      item.version_public_id,
      item,
    ]),
  );

  return (
    <details
      className="group mt-4 border border-slate-200"
      open={title === "Contexte inclus"}
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-sm font-semibold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600">
        <span>
          {title} <span className="text-slate-500">({items.length})</span>
        </span>
        <ChevronDown className="size-4 text-slate-400 transition-transform group-open:rotate-180" />
      </summary>
      <div className="divide-y divide-slate-100 border-t border-slate-200">
        {items.length ? (
          items.map((item) => (
            <article
              key={`${item.decision}-${item.candidate_public_id}`}
              className="p-4"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-sm font-semibold text-slate-900">
                  {includedKnowledge.get(item.candidate_public_id)?.title ??
                    `Source ${shortId(item.candidate_public_id)}`}
                </p>
                <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-indigo-600">
                  {includedKnowledge.get(item.candidate_public_id)?.entry_type
                    ? humanize(
                        includedKnowledge.get(item.candidate_public_id)!
                          .entry_type,
                      )
                    : item.is_mandatory
                      ? "Obligatoire"
                      : "Optionnelle"}
                </span>
              </div>
              <p className="mt-1 text-xs leading-5 text-slate-600">
                {item.explanation}
              </p>
              <p className="mt-2 font-mono text-[10px] text-slate-500">
                {humanize(item.reason_code)} ·{" "}
                {shortId(item.candidate_public_id)}
                {` · ${item.estimated_tokens} tokens`}
              </p>
            </article>
          ))
        ) : (
          <p className="p-4 text-sm text-slate-500">Aucune source.</p>
        )}
      </div>
    </details>
  );
}
