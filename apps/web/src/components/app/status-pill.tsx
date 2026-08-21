import {
  AlertTriangle,
  Check,
  CircleDashed,
  CircleX,
  ShieldAlert,
} from "lucide-react";

import { cn } from "@/lib/utils";

const toneByStatus: Record<string, string> = {
  active: "border-indigo-200 bg-indigo-50 text-indigo-700",
  passed: "border-emerald-200 bg-emerald-50 text-emerald-700",
  completed: "border-emerald-200 bg-emerald-50 text-emerald-700",
  committed: "border-emerald-200 bg-emerald-50 text-emerald-700",
  covered: "border-emerald-200 bg-emerald-50 text-emerald-700",
  resolved: "border-emerald-200 bg-emerald-50 text-emerald-700",
  partial: "border-amber-200 bg-amber-50 text-amber-800",
  warning: "border-amber-200 bg-amber-50 text-amber-800",
  passed_with_warning: "border-amber-200 bg-amber-50 text-amber-800",
  stale: "border-amber-200 bg-amber-50 text-amber-800",
  blocking: "border-red-200 bg-red-50 text-red-700",
  blocked: "border-red-200 bg-red-50 text-red-700",
  missing: "border-red-200 bg-red-50 text-red-700",
  open: "border-red-200 bg-red-50 text-red-700",
  proposed: "border-violet-200 bg-violet-50 text-violet-700",
  accepted: "border-violet-200 bg-violet-50 text-violet-700",
  rejected: "border-slate-200 bg-slate-50 text-slate-600",
  dismissed: "border-slate-200 bg-slate-50 text-slate-600",
  superseded: "border-slate-200 bg-slate-50 text-slate-600",
};

function Icon({ status }: { status: string }) {
  if (
    ["passed", "completed", "committed", "covered", "resolved"].includes(status)
  )
    return <Check />;
  if (["blocking", "blocked", "missing", "open"].includes(status))
    return <ShieldAlert />;
  if (["partial", "warning", "passed_with_warning", "stale"].includes(status))
    return <AlertTriangle />;
  if (["rejected", "dismissed"].includes(status)) return <CircleX />;
  return <CircleDashed />;
}

export function StatusPill({
  status,
  label,
}: {
  status: string;
  label?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex w-fit items-center gap-1.5 border px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.08em]",
        toneByStatus[status] ?? "border-slate-200 bg-slate-50 text-slate-600",
      )}
    >
      <span className="size-3 [&>svg]:size-3">
        <Icon status={status} />
      </span>
      {label ?? status.replaceAll("_", " ")}
    </span>
  );
}
