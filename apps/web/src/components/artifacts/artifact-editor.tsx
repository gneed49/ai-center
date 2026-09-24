import { typedDraft, draftMarkdown } from "./typed-draft";
import { TypedDraftEditor } from "./typed-draft-editor";
import { useState } from "react";
import type { FormEvent } from "react";
import type {
  ArtifactContent,
  ArtifactSource,
  ArtifactSourceInput,
} from "@/api/artifact-types";
import type { ProjectSnapshot, SessionView } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

export function ArtifactEditor({
  initial,
  snapshot,
  sourceSnapshots = [],
  conversation,
  busy,
  onSubmit,
  submitLabel,
}: {
  initial: ArtifactContent;
  snapshot?: ProjectSnapshot;
  sourceSnapshots?: ArtifactSource[];
  conversation?: SessionView;
  busy: boolean;
  onSubmit: (content: ArtifactContent) => void;
  submitLabel: string;
}) {
  const [draft, setDraft] = useState(() =>
    typedDraft(initial.structured_content),
  );
  const [title, setTitle] = useState(initial.title);
  const [body, setBody] = useState(initial.body_markdown);
  const [sources, setSources] = useState<ArtifactSourceInput[]>(() =>
    initial.sources.map(({ kind, public_id }) => ({ kind, public_id })),
  );
  const options = new Map<
    string,
    { source: ArtifactSourceInput; label: string }
  >();
  const add = (source: ArtifactSourceInput, label: string) =>
    options.set(`${source.kind}:${source.public_id}`, { source, label });
  for (const item of snapshot?.knowledge ?? [])
    add(
      { kind: "knowledge", public_id: item.version_public_id },
      `${item.title} · connaissance v${item.version_number}`,
    );
  for (const item of snapshot?.deliverables ?? [])
    add(
      { kind: "deliverable", public_id: item.public_id },
      `${item.title} · livrable v${item.version}`,
    );
  const pack = snapshot?.latest_handoff?.context_pack;
  if (pack)
    add(
      { kind: "context_pack", public_id: pack.public_id },
      `Contexte transmis · v${pack.version}${pack.status === "current" ? "" : " · obsolète"}`,
    );
  for (const item of sourceSnapshots)
    add(
      { kind: item.kind, public_id: item.public_id },
      `${item.title || item.kind}${item.version ? ` · v${item.version}` : ""} · source conservée`,
    );
  if (conversation)
    add(
      { kind: "session", public_id: conversation.session.public_id },
      `${conversation.session.title} · conversation d’origine`,
    );
  function toggle(source: ArtifactSourceInput) {
    const has = sources.some(
      (item) =>
        item.kind === source.kind && item.public_id === source.public_id,
    );
    setSources(
      has
        ? sources.filter(
            (item) =>
              item.kind !== source.kind || item.public_id !== source.public_id,
          )
        : [...sources, source],
    );
  }
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!title.trim() || busy) return;
    onSubmit({
      title: title.trim(),
      body_markdown: draft ? draftMarkdown(draft) : body,
      structured_content: draft
        ? {
            ...initial.structured_content,
            draft: { ...draft, title: title.trim() },
          }
        : initial.structured_content,
      sources,
    });
  }
  const lastResponse = conversation?.messages
    .filter((message) => message.role === "assistant")
    .at(-1);
  return (
    <form
      onSubmit={submit}
      className="space-y-6"
      aria-label="Édition du livrable"
      aria-busy={busy}
    >
      <div>
        <label
          htmlFor="artifact-title"
          className="mb-2 block text-sm font-medium"
        >
          Titre du livrable
        </label>
        <Input
          id="artifact-title"
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          maxLength={200}
          required
          disabled={busy}
        />
      </div>
      {draft ? (
        <TypedDraftEditor value={draft} onChange={setDraft} busy={busy} />
      ) : (
        <div>
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <label htmlFor="artifact-body" className="text-sm font-medium">
              Contenu
            </label>
            {lastResponse ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={busy}
                onClick={() => setBody(lastResponse.content)}
              >
                Reprendre la dernière réponse
              </Button>
            ) : null}
          </div>
          <Textarea
            id="artifact-body"
            rows={14}
            value={body}
            onChange={(event) => setBody(event.target.value)}
            disabled={busy}
            placeholder="Rédigez votre brouillon ou collez le texte à relire. Le format Markdown est accepté."
            className="min-h-72 leading-7"
          />
          <p className="mt-2 text-xs text-muted-foreground">
            Relisez le contenu et ses sources avant de valider cette version.
          </p>
        </div>
      )}
      <fieldset
        disabled={busy || Boolean(draft)}
        className="rounded-lg border p-4"
      >
        <legend className="px-1 text-sm font-medium">
          Sources de cette version
        </legend>
        {options.size ? (
          <div className="max-h-64 space-y-3 overflow-y-auto">
            {Array.from(options, ([key, option]) => (
              <label key={key} className="flex items-start gap-3 text-sm">
                <input
                  type="checkbox"
                  checked={sources.some(
                    (source) =>
                      source.kind === option.source.kind &&
                      source.public_id === option.source.public_id,
                  )}
                  onChange={() => toggle(option.source)}
                  className="mt-1 accent-indigo-600"
                />
                <span>{option.label}</span>
              </label>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            Aucune connaissance ou conversation disponible à rattacher. Vous
            pouvez conserver un brouillon rédigé manuellement.
          </p>
        )}
      </fieldset>
      <Button type="submit" disabled={busy || !title.trim()}>
        {busy ? "Enregistrement…" : submitLabel}
      </Button>
    </form>
  );
}
