import { Copy, Download, LoaderCircle } from "lucide-react";
import { useState } from "react";

import { api, type ContextPackExportFormat } from "@/api/client";
import {
  captureRequestContext,
  RequestContextChangedError,
} from "@/api/request-context";
import type { ContextPackSummary } from "@/api/types";
import { Button } from "@/components/ui/button";
import { isContextPackCurrent } from "@/lib/context-pack";
import { downloadContextPack } from "@/lib/context-pack-export";

export function ContextPackExport({ pack }: { pack: ContextPackSummary }) {
  const [busy, setBusy] = useState(false);
  const [outdated, setOutdated] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const current = isContextPackCurrent(pack) && !outdated;

  async function exportPack(format: ContextPackExportFormat, copy = false) {
    if (busy || !current) return;
    const context = captureRequestContext();
    setBusy(true);
    setNotice(null);
    setError(null);
    try {
      const freshPack = await api.contextPack(pack.public_id);
      context.assertCurrent();
      if (!isContextPackCurrent(freshPack)) {
        setOutdated(true);
        return;
      }
      const content = await api.exportContextPack(pack.public_id, format);
      context.assertCurrent();
      if (copy) {
        if (!navigator.clipboard?.writeText)
          throw new Error(
            "La copie est indisponible dans ce navigateur. Téléchargez le fichier Markdown.",
          );
        await navigator.clipboard.writeText(content);
        context.assertCurrent();
        setNotice("Contexte Markdown copié avec sa version et ses sources.");
      } else {
        downloadContextPack(freshPack, format, content);
        setNotice(
          "Téléchargement demandé. Conservez ce fichier avec sa version.",
        );
      }
    } catch (cause) {
      if (cause instanceof RequestContextChangedError) return;
      setError(
        cause instanceof Error
          ? cause.message
          : "L’export n’a pas abouti. Vous pouvez réessayer.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section
      className="space-y-4 border border-slate-200 bg-white p-5 sm:p-6"
      aria-labelledby="context-pack-export-title"
      aria-busy={busy}
    >
      <div>
        <h2 id="context-pack-export-title" className="font-semibold">
          Utiliser ce contexte dans un outil externe
        </h2>
        <p className="mt-2 text-sm text-slate-600">
          Copiez ou téléchargez le pack pour votre outil de travail. Ses
          sources, leurs versions et les choix de sélection accompagnent le
          contenu.
        </p>
        <p className="mt-2 break-all text-xs text-slate-500">
          Pack {pack.public_id} · version {pack.version} · contexte source v
          {pack.source_graph_version}
        </p>
      </div>
      {!current ? (
        <p role="alert" className="text-sm text-amber-800">
          Ce pack est obsolète. Recompilez le contexte avant de le transmettre à
          un outil externe. L’ancienne version reste consultable ci-dessous.
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button
          disabled={!current || busy}
          onClick={() => void exportPack("markdown", true)}
        >
          <Copy /> Copier le contexte
        </Button>
        <Button
          variant="outline"
          disabled={!current || busy}
          onClick={() => void exportPack("markdown")}
        >
          <Download /> Télécharger Markdown
        </Button>
        <Button
          variant="outline"
          disabled={!current || busy}
          onClick={() => void exportPack("json")}
        >
          <Download /> Télécharger JSON
        </Button>
      </div>
      {busy ? (
        <p
          role="status"
          className="flex items-center gap-2 text-sm text-slate-600"
        >
          <LoaderCircle className="size-4 animate-spin" /> Vérification du
          contexte et préparation de l’export…
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="text-sm text-emerald-800">
          {notice}
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="text-sm text-red-700">
          {error}
        </p>
      ) : null}
      <p className="text-xs text-slate-500">
        Après le travail externe, rattachez la référence GitHub et le pack
        transmis dans les preuves du livrable. L’export ne vaut pas validation
        du travail réalisé.
      </p>
    </section>
  );
}
