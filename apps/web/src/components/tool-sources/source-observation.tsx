import { Link } from "react-router";
import type { SourceObservationDetail } from "@/api/tool-source-types";
import { observationPath } from "@/api/tool-sources";
import {
  coverageLabels,
  freshnessLabels,
  omissionLabels,
  safeSourceUrl,
  sourceDate,
  sourceProviderLabels,
  sourceTitle,
} from "./source-labels";

export function SourceObservationView({
  detail,
}: {
  detail: SourceObservationDetail;
}) {
  const o = detail.observation;
  const url = safeSourceUrl(o.canonical_url);
  const current = o.freshness.current_observation_id;
  const currentPath = current ? observationPath(o.source_kind, current) : null;
  const state = detail.metadata.state;
  const stateName =
    state &&
    typeof state === "object" &&
    "name" in state &&
    typeof state.name === "string"
      ? state.name
      : null;
  return (
    <article
      className="min-w-0 space-y-5 break-words rounded-lg border bg-white p-5 sm:p-6"
      aria-label="Contenu de la lecture"
    >
      <div className="flex flex-wrap gap-2 text-xs font-medium">
        <span className="rounded-full bg-indigo-50 px-3 py-1 text-indigo-900">
          Source observée
        </span>
        <span className="rounded-full bg-slate-100 px-3 py-1">
          {sourceProviderLabels[o.provider]} · Lecture enregistrée n°{o.version}
        </span>
      </div>
      <h2 className="text-xl font-semibold">{sourceTitle(o.title)}</h2>
      <p className="text-sm text-muted-foreground">
        Cette lecture conserve ce qui a été reçu de l’outil. Elle ne constitue
        pas une règle validée ni une preuve de réalisation.
      </p>
      <dl className="grid gap-4 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-muted-foreground">Contenu lu le</dt>
          <dd>{sourceDate(o.observed_at)}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Dernière vérification le</dt>
          <dd>{sourceDate(o.freshness.last_checked_at)}</dd>
        </div>
        {o.remote_updated_at ? (
          <div>
            <dt className="text-muted-foreground">Modifié dans l’outil le</dt>
            <dd>{sourceDate(o.remote_updated_at)}</dd>
          </div>
        ) : null}
        {stateName ? (
          <div>
            <dt className="text-muted-foreground">État dans Linear</dt>
            <dd>{stateName}</dd>
          </div>
        ) : null}
      </dl>
      {o.freshness.last_check_status === "failed" ? (
        <p
          role="status"
          className="rounded border border-amber-200 bg-amber-50 p-3 text-sm text-amber-950"
        >
          La dernière vérification n’a pas abouti. Le contenu ci-dessous
          conserve sa date de lecture précédente.
        </p>
      ) : null}
      {o.freshness.reasons.length ? (
        <div className="space-y-2 rounded border border-amber-200 bg-amber-50 p-3 text-sm text-amber-950">
          {o.freshness.reasons.map((reason) => (
            <p key={reason}>
              {freshnessLabels[reason] ??
                "Cette source ne peut pas alimenter le contexte actuel."}
            </p>
          ))}
          {!o.freshness.is_current && currentPath ? (
            <Link className="block underline" to={currentPath}>
              Voir la lecture actuelle
            </Link>
          ) : null}
        </div>
      ) : null}
      <section
        className="space-y-2 rounded-md bg-slate-50 p-4 text-sm"
        aria-label="Couverture de cette lecture"
      >
        <h3 className="font-semibold">Ce que cette lecture couvre</h3>
        <p>{coverageLabels[o.coverage]}</p>
        {o.omission_reasons.length ? (
          <ul className="list-disc space-y-1 pl-5">
            {o.omission_reasons.map((reason) => (
              <li key={reason}>
                {omissionLabels[reason] ??
                  "Une limite de lecture supplémentaire est signalée."}
              </li>
            ))}
          </ul>
        ) : null}
        <p className="text-xs text-muted-foreground">
          Les parties non lues ne permettent pas de conclure qu’un contenu
          n’existe pas.
        </p>
      </section>
      {o.availability === "available" ? (
        <div className="whitespace-pre-wrap text-sm leading-7">
          {detail.body_markdown || "Cette lecture ne contient pas de texte."}
        </div>
      ) : (
        <p role="status">
          Source absente ou inaccessible dans l’outil. Aucun ancien texte n’est
          présenté comme une nouvelle lecture.
        </p>
      )}
      <nav
        className="flex flex-wrap gap-4 text-sm text-primary"
        aria-label="Origine de la lecture"
      >
        {url ? (
          <a href={url} target="_blank" rel="noreferrer">
            Ouvrir dans {sourceProviderLabels[o.provider]}
          </a>
        ) : null}
        {o.reference_public_id ? (
          <Link to={`/sources/${o.reference_public_id}`}>
            Voir la source et son historique
          </Link>
        ) : (
          <span>Créée depuis un livrable · origine publication conservée</span>
        )}
      </nav>
      <details className="border-t pt-3 text-xs text-muted-foreground">
        <summary>Détails techniques de la lecture</summary>
        <dl className="mt-3 space-y-2 break-all">
          <div>
            <dt>Identifiant exact</dt>
            <dd>{o.public_id}</dd>
          </div>
          <div>
            <dt>Type de source</dt>
            <dd>{o.source_kind}</dd>
          </div>
          <div>
            <dt>Empreinte du contenu</dt>
            <dd>{o.content_hash}</dd>
          </div>
          <div>
            <dt>Empreinte de la lecture</dt>
            <dd>{o.snapshot_hash}</dd>
          </div>
          {o.publication_public_id ? (
            <div>
              <dt>Publication d’origine</dt>
              <dd>{o.publication_public_id}</dd>
            </div>
          ) : null}
          <div>
            <dt>Limites déclarées</dt>
            <dd>{o.omission_reasons.join(", ") || "Aucune limite déclarée"}</dd>
          </div>
        </dl>
      </details>
    </article>
  );
}
