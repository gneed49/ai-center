import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { workToolsApi } from "@/api/work-tools";
import { Button } from "@/components/ui/button";
import { LoadingState, ErrorState } from "@/components/app/page";
import { PublicationCard } from "./publication-card";
export function TicketHistory({
  artifactId,
  currentVersionId,
  canEdit,
}: {
  artifactId: string;
  currentVersionId: string;
  canEdit: boolean;
}) {
  const [offset, setOffset] = useState(0);
  const [open, setOpen] = useState(false);
  const history = useQuery({
    queryKey: ["publications", artifactId, offset],
    queryFn: () => workToolsApi.publications(artifactId, offset),
    enabled: open,
  });
  return (
    <details
      className="space-y-4 rounded-lg border p-5"
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="font-medium">Historique des publications</summary>
      {open ? (
        history.isPending ? (
          <LoadingState />
        ) : history.error ? (
          <ErrorState
            error={history.error}
            retry={() => void history.refetch()}
          />
        ) : (
          <>
            <div className="grid gap-3">
              {history.data.items.map((job) => (
                <PublicationCard
                  key={job.public_id}
                  publication={job}
                  currentVersionId={currentVersionId}
                  canEdit={canEdit}
                />
              ))}
            </div>
            {!history.data.items.length ? (
              <p className="text-sm">Aucune publication enregistrée.</p>
            ) : null}
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 10))}
              >
                Plus récentes
              </Button>
              <Button
                variant="outline"
                disabled={history.data.items.length < 10}
                onClick={() => setOffset(offset + 10)}
              >
                Plus anciennes
              </Button>
            </div>
          </>
        )
      ) : null}
    </details>
  );
}
