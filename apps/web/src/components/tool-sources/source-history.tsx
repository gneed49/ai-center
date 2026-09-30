import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { observationPath, toolSourcesApi } from "@/api/tool-sources";
import { Button } from "@/components/ui/button";
import { ErrorState, LoadingState } from "@/components/app/page";
import { coverageLabels, sourceDate, sourceTitle } from "./source-labels";
export function SourceHistory({ referenceId }: { referenceId: string }) {
  const [open, setOpen] = useState(false);
  const [cursors, setCursors] = useState<string[]>([]);
  const cursor = cursors.at(-1);
  const history = useQuery({
    queryKey: ["tool-source-history", referenceId, cursor],
    queryFn: () => toolSourcesApi.history(referenceId, cursor),
    enabled: open,
    retry: false,
  });
  return (
    <details
      className="rounded-lg border p-5"
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="font-semibold">Historique des lectures</summary>
      {open &&
        (history.isPending ? (
          <LoadingState label="Chargement de l’historique…" />
        ) : history.error ? (
          <ErrorState
            error={history.error}
            retry={() => void history.refetch()}
          />
        ) : (
          <div className="mt-4 space-y-4">
            <p className="text-sm">
              {history.data.total_count} lecture(s) enregistrée(s). Une
              vérification inchangée conserve la même lecture.
            </p>
            <ul className="space-y-3">
              {history.data.items.map((o) => (
                <li key={o.public_id} className="rounded border p-3 text-sm">
                  <Link
                    className="font-medium text-primary"
                    to={observationPath(o.source_kind, o.public_id)!}
                  >
                    Lecture n°{o.version} · {sourceTitle(o.title)}
                  </Link>
                  <p>
                    {sourceDate(o.observed_at)} · {coverageLabels[o.coverage]}
                  </p>
                </li>
              ))}
            </ul>
            <div className="flex gap-3">
              <Button
                variant="outline"
                disabled={!cursors.length}
                onClick={() => setCursors((v) => v.slice(0, -1))}
              >
                Plus récentes
              </Button>
              <Button
                variant="outline"
                disabled={!history.data.next_cursor}
                onClick={() =>
                  setCursors((v) => [...v, history.data.next_cursor!])
                }
              >
                Plus anciennes
              </Button>
            </div>
          </div>
        ))}
    </details>
  );
}
