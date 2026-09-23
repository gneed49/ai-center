import { useQuery } from "@tanstack/react-query";
import { request } from "@/api/client";
import { Button } from "@/components/ui/button";

type Coverage = {
  available_sources: number;
  pending_sources: number;
  examined_pairs: number;
  omitted_neighbors: number;
  max_sources: number;
  max_neighbors_per_source: number;
  max_provider_calls_per_hour: number;
  progress: {
    status: string;
    last_error_code: string | null;
    lease_expired: boolean | null;
  } | null;
};

export function StewardProgress() {
  const coverage = useQuery({
    queryKey: ["steward-progress"],
    queryFn: () => request<Coverage>("/api/company/steward"),
    refetchInterval: 30_000,
  });
  if (coverage.isPending)
    return (
      <p className="text-sm text-muted-foreground">
        Vérification de l’analyse en cours…
      </p>
    );
  if (coverage.isError)
    return (
      <div className="flex items-center gap-3 text-sm">
        <p>La progression de l’analyse est indisponible.</p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void coverage.refetch()}
        >
          Réessayer
        </Button>
      </div>
    );
  const data = coverage.data;
  const blocked = data.available_sources > data.max_sources;
  return (
    <section
      aria-label="Progression de l’analyse de cohérence"
      className="space-y-2 rounded-lg border bg-muted/30 p-4 text-sm"
    >
      <h2 className="font-semibold">
        {blocked
          ? "L’analyse a atteint sa limite de sources"
          : data.pending_sources > 0
            ? `${data.pending_sources} source(s) à examiner`
            : "Les sources disponibles ont été parcourues"}
      </h2>
      <p className="text-muted-foreground">
        {data.available_sources} source(s) disponibles · {data.examined_pairs}{" "}
        comparaison(s) effectuée(s). Chaque nouvelle version est examinée avec
        au plus {data.max_neighbors_per_source} voisins pertinents.
      </p>
      <p className="text-muted-foreground">
        Les liens sans termes communs, les contenus non lus et les voisins omis
        restent hors de cette analyse. Un parcours terminé ne garantit pas
        l’absence de contradictions.
      </p>
      {data.omitted_neighbors > 0 && (
        <p>
          {data.omitted_neighbors} voisin(s) omis dans l’historique des
          sélections.
        </p>
      )}
      <p className="text-xs text-muted-foreground">
        Maximum : {data.max_provider_calls_per_hour} appels d’analyse par heure
        pour l’entreprise, en plus des limites IA habituelles.
        {blocked
          ? ` Le catalogue dépasse ${data.max_sources} sources ; l’opérateur doit revoir son périmètre.`
          : " La suite reprend automatiquement selon les accès, les quotas et l’activation de l’analyse."}
      </p>
    </section>
  );
}
