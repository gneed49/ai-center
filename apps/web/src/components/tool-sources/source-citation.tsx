import { Link } from "react-router";
import { observationPath } from "@/api/tool-sources";
import type { SourceObservation } from "@/api/tool-source-types";
import {
  coverageLabels,
  omissionLabels,
  sourceDate,
  sourceTitle,
} from "./source-labels";
export function SourceCitation({
  kind,
  publicId,
  title,
  observation,
}: {
  kind: string;
  publicId: string;
  title?: string;
  observation?: Partial<SourceObservation>;
}) {
  const path = observationPath(kind, publicId);
  if (!path || (observation?.public_id && observation.public_id !== publicId))
    return (
      <p className="text-sm text-amber-900">
        Cette source ne dispose pas d’un lien de lecture exact reconnu.
      </p>
    );
  return (
    <div className="space-y-2 break-words rounded border bg-slate-50 p-3 text-sm">
      <p className="text-xs font-medium text-indigo-800">
        Source observée
        {observation?.provider
          ? ` · ${observation.provider === "linear" ? "Linear" : "Notion"}`
          : ""}
      </p>
      <Link to={path} className="block font-medium text-primary underline">
        {sourceTitle(title ?? observation?.title ?? "")} · ouvrir la lecture
        exacte
      </Link>
      {observation?.observed_at ? (
        <p className="text-xs">
          Contenu lu le {sourceDate(observation.observed_at)}
          {observation.version ? ` · lecture n°${observation.version}` : ""}
        </p>
      ) : null}
      {observation?.coverage ? (
        <p className="text-xs">{coverageLabels[observation.coverage]}</p>
      ) : null}
      {observation?.omission_reasons?.length ? (
        <ul className="list-disc pl-4 text-xs">
          {observation.omission_reasons.map((reason) => (
            <li key={reason}>
              {omissionLabels[reason] ??
                "Une limite supplémentaire est signalée."}
            </li>
          ))}
        </ul>
      ) : null}
      <p className="text-xs text-muted-foreground">
        Cette citation conserve sa lecture d’origine. L’état actuel et les
        droits sont vérifiés à l’ouverture.
      </p>
    </div>
  );
}
