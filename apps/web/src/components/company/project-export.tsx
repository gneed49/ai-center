import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Download } from "lucide-react";
import { companyApi } from "@/api/company";
import { companyControlsApi } from "@/api/company-controls";
import {
  captureRequestContext,
  RequestContextChangedError,
} from "@/api/request-context";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/app/page";

export function ProjectExport({ projectId }: { projectId: string }) {
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);
  async function download() {
    if (busy) return;
    const context = captureRequestContext();
    setBusy(true);
    setNotice(null);
    setError(null);
    try {
      const text = await companyControlsApi.exportProject(projectId);
      context.assertCurrent();
      const url = URL.createObjectURL(
        new Blob([text], { type: "application/json;charset=utf-8" }),
      );
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `ai-center-project-${projectId}.json`;
      document.body.append(anchor);
      try {
        anchor.click();
      } finally {
        anchor.remove();
        setTimeout(() => URL.revokeObjectURL(url), 0);
      }
      setNotice(
        "Téléchargement de l’export du projet demandé. Le fichier contient ses données, les versions sources citées et les exclusions.",
      );
    } catch (cause) {
      if (!(cause instanceof RequestContextChangedError))
        setError(
          cause instanceof Error
            ? cause
            : new Error("L’export du projet n’a pas pu être préparé."),
        );
    } finally {
      setBusy(false);
    }
  }
  if (
    company.data?.workspace.role !== "owner" ||
    company.data.company_scope?.project_public_id === projectId
  )
    return null;
  return (
    <section className="max-w-xl space-y-3" aria-label="Export du projet">
      <Button variant="outline" disabled={busy} onClick={() => void download()}>
        <Download />
        {busy ? "Préparation de l’export…" : "Exporter ce projet"}
      </Button>
      {notice ? (
        <p role="status" className="text-sm text-emerald-800">
          {notice}
        </p>
      ) : null}
      {error ? (
        <ErrorState title="L’export du projet n’a pas abouti" error={error} />
      ) : null}
    </section>
  );
}
