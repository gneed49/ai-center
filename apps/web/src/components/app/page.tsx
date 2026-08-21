import type { ReactNode } from "react";
import { AlertCircle, LoaderCircle } from "lucide-react";

import { Button } from "@/components/ui/button";

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="flex flex-col gap-5 border-b border-slate-200 pb-7 sm:flex-row sm:items-end sm:justify-between">
      <div className="max-w-3xl">
        {eyebrow ? (
          <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-indigo-600">
            {eyebrow}
          </p>
        ) : null}
        <h1 className="text-balance text-3xl font-semibold tracking-[-0.035em] text-slate-950 sm:text-4xl">
          {title}
        </h1>
        {description ? (
          <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-600 sm:text-base">
            {description}
          </p>
        ) : null}
      </div>
      {actions ? (
        <div className="flex shrink-0 flex-wrap gap-2">{actions}</div>
      ) : null}
    </header>
  );
}

export function LoadingState({
  label = "Chargement du contexte…",
}: {
  label?: string;
}) {
  return (
    <div className="grid min-h-64 place-items-center border border-dashed border-slate-300 bg-slate-50/60">
      <div className="flex items-center gap-3 text-sm text-slate-600">
        <LoaderCircle className="size-4 animate-spin text-indigo-600" />
        {label}
      </div>
    </div>
  );
}

export function ErrorState({
  error,
  retry,
}: {
  error: Error;
  retry?: () => void;
}) {
  return (
    <div className="border border-red-200 bg-red-50 p-6 text-red-900">
      <div className="flex items-start gap-3">
        <AlertCircle className="mt-0.5 size-5 shrink-0" />
        <div>
          <p className="font-semibold">
            Le serveur n’a pas pu terminer cette action.
          </p>
          <p className="mt-1 text-sm text-red-700">{error.message}</p>
          {retry ? (
            <Button
              variant="outline"
              size="sm"
              className="mt-4 border-red-300 bg-white"
              onClick={retry}
            >
              Réessayer
            </Button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="grid min-h-56 place-items-center border border-dashed border-slate-300 bg-slate-50/60 p-8 text-center">
      <div className="max-w-sm">
        <p className="font-semibold text-slate-900">{title}</p>
        <p className="mt-2 text-sm leading-6 text-slate-600">{description}</p>
        {action ? (
          <div className="mt-5 flex justify-center">{action}</div>
        ) : null}
      </div>
    </div>
  );
}
