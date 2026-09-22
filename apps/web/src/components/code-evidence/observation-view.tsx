import { useState } from "react";
import { Copy } from "lucide-react";
import type {
  CodeFileObservation,
  CodeObservationDetail,
} from "@/api/code-observations";
import { codeObservationLabel, githubCodeUrl } from "@/lib/code-evidence";
import { formatDate } from "@/lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function CodeObservationView({
  detail,
}: {
  detail: CodeObservationDetail;
}) {
  const [fileId, setFileId] = useState(detail.files[0]?.public_id ?? "");
  const file =
    detail.files.find((item) => item.public_id === fileId) ?? detail.files[0];
  return (
    <section className="space-y-5" aria-labelledby="code-observation-title">
      <header className="rounded-lg border p-5">
        <h2 id="code-observation-title" className="text-xl font-semibold">
          {detail.corpus.repository}
        </h2>
        <p className="mt-2 break-all font-mono text-xs text-muted-foreground">
          Commit {detail.corpus.commit_sha}
        </p>
        <p className="mt-2 text-sm text-muted-foreground">
          Observé le {formatDate(detail.corpus.observed_at, true)} ·{" "}
          {detail.corpus.commit_verified
            ? "Commit vérifié"
            : "Commit non vérifié"}
        </p>
        <p className="mt-3 text-sm leading-6 text-amber-900">
          Cette observation porte uniquement sur les fichiers sélectionnés. Elle
          ne vérifie pas le fonctionnement du dépôt entier et n’exécute pas son
          code.
        </p>
      </header>
      {detail.files.length ? (
        <div className="grid gap-5 xl:grid-cols-[240px_minmax(0,1fr)]">
          <nav className="space-y-2" aria-label="Fichiers observés">
            {detail.files.map((item) => (
              <button
                key={item.public_id}
                className={`w-full rounded-md border p-3 text-left text-sm ${file?.public_id === item.public_id ? "border-primary/30 bg-primary/5" : "hover:bg-muted"}`}
                onClick={() => setFileId(item.public_id)}
                aria-current={
                  file?.public_id === item.public_id ? "true" : undefined
                }
              >
                <span className="block break-all font-medium">{item.path}</span>
                <span className="mt-1 block text-xs text-muted-foreground">
                  {codeObservationLabel(item.status, item.reason_code)}
                </span>
              </button>
            ))}
          </nav>
          {file ? (
            <ObservedFile
              key={file.public_id}
              file={file}
              repository={detail.corpus.repository}
              commit={detail.corpus.commit_sha}
            />
          ) : null}
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">
          Aucun fichier observé dans cette capture.
        </p>
      )}
    </section>
  );
}
function ObservedFile({
  file,
  repository,
  commit,
}: {
  file: CodeFileObservation;
  repository: string;
  commit: string;
}) {
  const [from, setFrom] = useState("1");
  const [to, setTo] = useState(String(Math.max(1, file.line_count)));
  const [notice, setNotice] = useState<string | null>(null);
  const lineStart = Number(from),
    lineEnd = Number(to);
  const validRange =
    Number.isInteger(lineStart) &&
    Number.isInteger(lineEnd) &&
    lineStart > 0 &&
    lineEnd >= lineStart &&
    lineEnd <= file.line_count;
  const url = githubCodeUrl(
    repository,
    commit,
    file.path,
    validRange ? lineStart : undefined,
    validRange ? lineEnd : undefined,
  );
  async function copy() {
    if (!validRange || !url) return;
    try {
      if (!navigator.clipboard?.writeText) throw new Error();
      await navigator.clipboard.writeText(
        `${repository}@${commit}:${file.path}:L${lineStart}-L${lineEnd}\n${url}\nObservation AI Center ${file.public_id} · ${file.observed_at}`,
      );
      setNotice("Référence exacte copiée.");
    } catch {
      setNotice("Copie indisponible. Utilisez le lien vers ces lignes.");
    }
  }
  const textReadable =
    file.status === "code_read" && file.content_text !== null;
  return (
    <article className="min-w-0 space-y-4 rounded-lg border p-5">
      <header>
        <h3 className="break-all font-semibold">{file.path}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {codeObservationLabel(file.status, file.reason_code)}
          {textReadable ? ` · ${file.line_count} ligne(s)` : ""}
        </p>
      </header>
      {textReadable ? (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <div>
              <label
                htmlFor={`code-line-from-${file.public_id}`}
                className="mb-1 block text-xs"
              >
                Première ligne
              </label>
              <Input
                id={`code-line-from-${file.public_id}`}
                className="w-28"
                type="number"
                min={1}
                max={file.line_count}
                value={from}
                onChange={(event) => setFrom(event.target.value)}
              />
            </div>
            <div>
              <label
                htmlFor={`code-line-to-${file.public_id}`}
                className="mb-1 block text-xs"
              >
                Dernière ligne
              </label>
              <Input
                id={`code-line-to-${file.public_id}`}
                className="w-28"
                type="number"
                min={1}
                max={file.line_count}
                value={to}
                onChange={(event) => setTo(event.target.value)}
              />
            </div>
            <Button
              variant="outline"
              disabled={!validRange}
              onClick={() => void copy()}
            >
              <Copy />
              Copier la référence
            </Button>
            {url && validRange ? (
              <a
                className="text-sm text-primary underline"
                href={url}
                target="_blank"
                rel="noopener noreferrer"
              >
                Voir ces lignes sur GitHub
              </a>
            ) : null}
          </div>
          {notice ? (
            <p role="status" className="text-sm text-muted-foreground">
              {notice}
            </p>
          ) : null}
          <div className="max-h-[36rem] overflow-auto rounded-md border bg-muted/30 p-4">
            <ol className="min-w-max font-mono text-xs leading-6">
              {file
                .content_text!.split("\n")
                .slice(0, file.line_count)
                .map((line, index) => (
                  <li
                    key={index}
                    className={`flex gap-5 ${validRange && index + 1 >= lineStart && index + 1 <= lineEnd ? "bg-primary/5" : ""}`}
                  >
                    <span
                      className="sticky left-0 w-8 shrink-0 select-none bg-muted text-right text-muted-foreground"
                      aria-label={`Ligne ${index + 1}`}
                    >
                      {index + 1}
                    </span>
                    <code className="whitespace-pre">{line || " "}</code>
                  </li>
                ))}
            </ol>
          </div>
        </>
      ) : (
        <p className="rounded-md bg-amber-50 p-4 text-sm text-amber-950">
          {file.reason_code === "sensitive_content_excluded"
            ? "Le texte a été exclu de l’observation car il peut contenir une clé ou un identifiant d’accès sensible."
            : "Le contenu de ce fichier n’a pas été lu. Ce résultat ne permet pas de conclure à l’absence d’une fonctionnalité ou d’une règle."}
        </p>
      )}
      <details className="text-xs text-muted-foreground">
        <summary>Provenance de cette observation</summary>
        <dl className="mt-3 space-y-2 break-all">
          <div>
            <dt>Observation</dt>
            <dd>{file.public_id}</dd>
          </div>
          {file.blob_sha ? (
            <div>
              <dt>Objet Git</dt>
              <dd>{file.blob_sha}</dd>
            </div>
          ) : null}
          {file.content_hash ? (
            <div>
              <dt>Empreinte du texte</dt>
              <dd>{file.content_hash}</dd>
            </div>
          ) : null}
          {file.reason_code ? (
            <div>
              <dt>Motif fourni</dt>
              <dd>{file.reason_code}</dd>
            </div>
          ) : null}
        </dl>
      </details>
    </article>
  );
}
