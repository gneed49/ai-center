import { useState } from "react";
import { Copy, Download } from "lucide-react";
import { artifactsApi } from "@/api/artifacts";
import type { ArtifactVersion } from "@/api/artifact-types";
import {
  captureRequestContext,
  RequestContextChangedError,
} from "@/api/request-context";
import { Button } from "@/components/ui/button";

export function ArtifactExport({
  artifactId,
  version,
}: {
  artifactId: string;
  version: ArtifactVersion;
}) {
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  async function exportVersion(format: "markdown" | "json", copy = false) {
    const context = captureRequestContext();
    setBusy(true);
    setNotice(null);
    setError(null);
    try {
      const content = await artifactsApi.export(
        artifactId,
        version.public_id,
        format,
      );
      context.assertCurrent();
      if (copy) {
        if (!navigator.clipboard?.writeText)
          throw new Error(
            "La copie est indisponible. Vous pouvez télécharger le document.",
          );
        await navigator.clipboard.writeText(content);
        context.assertCurrent();
        setNotice(`Version ${version.version} copiée avec ses sources.`);
      } else {
        const url = URL.createObjectURL(
          new Blob([content], {
            type:
              format === "json"
                ? "application/json;charset=utf-8"
                : "text/markdown;charset=utf-8",
          }),
        );
        const link = document.createElement("a");
        link.href = url;
        link.download = `artifact-${artifactId}-v${version.version}.${format === "json" ? "json" : "md"}`;
        document.body.append(link);
        try {
          link.click();
        } finally {
          link.remove();
          setTimeout(() => URL.revokeObjectURL(url), 0);
        }
        setNotice(`Téléchargement de la version ${version.version} demandé.`);
      }
    } catch (cause) {
      if (!(cause instanceof RequestContextChangedError))
        setError(
          cause instanceof Error ? cause.message : "L’export n’a pas abouti.",
        );
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          disabled={busy}
          onClick={() => void exportVersion("markdown", true)}
        >
          <Copy /> Copier v{version.version}
        </Button>
        <Button
          variant="outline"
          disabled={busy}
          onClick={() => void exportVersion("markdown")}
        >
          <Download /> Markdown
        </Button>
        <Button
          variant="outline"
          disabled={busy}
          onClick={() => void exportVersion("json")}
        >
          <Download /> JSON
        </Button>
      </div>
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
    </div>
  );
}
