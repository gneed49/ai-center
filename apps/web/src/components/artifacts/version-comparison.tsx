import type { ArtifactVersion } from "@/api/artifact-types";

export function VersionComparison({
  before,
  after,
}: {
  before: ArtifactVersion;
  after: ArtifactVersion;
}) {
  const beforeLines = before.body_markdown.split("\n");
  const afterLines = after.body_markdown.split("\n");
  const beforeSet = new Set(beforeLines);
  const afterSet = new Set(afterLines);
  return (
    <section className="space-y-4" aria-labelledby="version-comparison-title">
      <h2 id="version-comparison-title" className="text-xl font-semibold">
        Comparer les versions {before.version} et {after.version}
      </h2>
      <p className="text-sm text-muted-foreground">
        Les lignes présentes dans une seule version sont surlignées. Un
        déplacement ou une répétition de ligne peut nécessiter une lecture des
        deux textes.
      </p>
      <div className="grid gap-4 lg:grid-cols-2">
        {[
          {
            version: before,
            lines: beforeLines,
            other: afterSet,
            tone: "bg-rose-50 text-rose-950",
          },
          {
            version: after,
            lines: afterLines,
            other: beforeSet,
            tone: "bg-emerald-50 text-emerald-950",
          },
        ].map(({ version, lines, other, tone }) => (
          <div key={version.public_id} className="min-w-0 rounded-lg border">
            <header className="border-b p-4">
              <h3 className="font-semibold">
                Version {version.version} · {version.title}
              </h3>
              <p className="mt-1 text-xs text-muted-foreground">
                {version.status === "validated" ? "Validée" : "Brouillon"} ·{" "}
                {version.sources.length} source(s)
              </p>
            </header>
            <pre className="overflow-x-auto whitespace-pre-wrap break-words p-4 text-sm leading-7">
              {lines.map((line, index) => (
                <span
                  key={index}
                  className={`block min-h-7 ${other.has(line) ? "" : tone}`}
                >
                  {line || "\u00a0"}
                </span>
              ))}
            </pre>
            {Object.keys(version.structured_content).length ? (
              <details className="border-t p-4 text-sm">
                <summary>Données structurées de cette version</summary>
                <pre className="mt-3 overflow-x-auto whitespace-pre-wrap break-words text-xs">
                  {JSON.stringify(version.structured_content, null, 2)}
                </pre>
              </details>
            ) : null}
            <div className="border-t p-4">
              <h4 className="text-sm font-medium">Sources conservées</h4>
              <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
                {version.sources.map((source) => (
                  <li
                    key={`${source.kind}:${source.public_id}`}
                    className="break-all"
                  >
                    {source.title} · {source.public_id}
                    {source.version ? ` · v${source.version}` : ""}
                  </li>
                ))}
                {!version.sources.length ? (
                  <li>Aucune source rattachée.</li>
                ) : null}
              </ul>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
