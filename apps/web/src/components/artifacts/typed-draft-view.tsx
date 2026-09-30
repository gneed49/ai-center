import { useEffect, useRef } from "react";
import { sectionLabel, ticketIndex, typedDraft } from "./typed-draft";
import type { ArtifactVersion } from "@/api/artifact-types";

export function TypedDraftView({
  version,
  ticket,
}: {
  version: ArtifactVersion;
  ticket: string | null;
}) {
  const draft = typedDraft(version.structured_content);
  const index = ticketIndex(ticket);
  const valid = index !== null && Boolean(draft?.tickets[index]);
  const focused = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (valid) {
      focused.current?.focus();
      focused.current?.scrollIntoView?.({
        block: "center",
        behavior: "instant",
      });
    }
  }, [valid, index, version.public_id]);
  return (
    <div className="space-y-6 break-words text-sm leading-7">
      {ticket !== null && !valid ? (
        <p
          role="status"
          className="rounded-md border border-amber-200 bg-amber-50 p-3 text-amber-950"
        >
          Ce ticket n’existe pas dans cette version. Consultez les entrées
          ci-dessous ou ouvrez une autre version.
        </p>
      ) : null}
      {!draft ? (
        <div className="whitespace-pre-wrap">
          {version.body_markdown ||
            "Ce brouillon ne contient pas encore de texte."}
        </div>
      ) : (
        <>
          <p className="whitespace-pre-wrap">{draft.summary}</p>
          {draft.sections.map((section) => (
            <section key={section.key}>
              <h3 className="mb-2 text-base font-semibold">
                {sectionLabel(section.key, section.title)}
              </h3>
              <p className="whitespace-pre-wrap">{section.body}</p>
            </section>
          ))}
          {draft.tickets.map((item, i) => (
            <article
              id={`ticket-${i}`}
              key={i}
              ref={i === index ? focused : undefined}
              tabIndex={i === index ? -1 : undefined}
              aria-label={`Ticket ${i + 1} · version ${version.version}`}
              className={`space-y-3 rounded-lg border p-4 outline-offset-4 ${i === index ? "border-primary bg-primary/5" : ""}`}
            >
              <p className="text-xs font-medium text-muted-foreground">
                Ticket {i + 1} · version {version.version}
              </p>
              <h3 className="text-base font-semibold">{item.title}</h3>
              <p className="whitespace-pre-wrap">{item.description}</p>
              <h4 className="font-medium">Critères d’acceptation</h4>
              <ul className="list-disc space-y-1 pl-5">
                {item.acceptance_criteria.map((criterion, n) => (
                  <li key={n}>{criterion}</li>
                ))}
              </ul>
            </article>
          ))}
          {draft.open_questions.length ? (
            <section>
              <h3 className="mb-2 text-base font-semibold">
                Points à clarifier
              </h3>
              <ul className="list-disc space-y-1 pl-5">
                {draft.open_questions.map((question, i) => (
                  <li key={i}>{question}</li>
                ))}
              </ul>
            </section>
          ) : null}
        </>
      )}
    </div>
  );
}
