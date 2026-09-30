import { useLayoutEffect, useRef, useState } from "react";
import type { ArtifactType } from "@/api/artifact-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { sectionLabel, type TypedDraft } from "./typed-draft";
import type { DraftErrors } from "./draft-edit-validation";
type Ticket = TypedDraft["tickets"][number];
export function TypedDraftEditor({
  value,
  onChange,
  busy,
  artifactType,
  errors = {},
  submission = 0,
}: {
  value: TypedDraft;
  onChange: (value: TypedDraft) => void;
  busy: boolean;
  artifactType?: ArtifactType;
  errors?: DraftErrors;
  submission?: number;
}) {
  const canChangeCount =
    artifactType === "product_tickets" || artifactType === "technical_tickets";
  const [keys, setKeys] = useState<string[]>(() =>
    value.tickets.map(() => crypto.randomUUID()),
  );
  const [undo, setUndo] = useState<{
    index: number;
    key: string;
    ticket: Ticket;
    submission: number;
  } | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const titleInputs = useRef(new Map<string, HTMLInputElement>());
  const focusNext = useRef<string | null>(null);
  useLayoutEffect(() => {
    if (focusNext.current) titleInputs.current.get(focusNext.current)?.focus();
    focusNext.current = null;
  }, [keys]);
  function add() {
    if (busy || !canChangeCount || value.tickets.length >= 30) return;
    const key = crypto.randomUUID();
    focusNext.current = key;
    setKeys([...keys, key]);
    setUndo(null);
    onChange({
      ...value,
      tickets: [
        ...value.tickets,
        {
          title: "",
          description: "",
          acceptance_criteria: [""],
          source_ids: [],
        },
      ],
    });
    setAnnouncement(
      `Ticket ${value.tickets.length + 1} ajouté. Complétez son titre, sa description et ses critères.`,
    );
  }
  function remove(index: number) {
    if (busy || !canChangeCount || value.tickets.length <= 1) return;
    setUndo({
      index,
      key: keys[index],
      ticket: value.tickets[index],
      submission,
    });
    focusNext.current = keys[index + 1] ?? keys[index - 1];
    setKeys(keys.filter((_, i) => i !== index));
    onChange({
      ...value,
      tickets: value.tickets.filter((_, i) => i !== index),
    });
    setAnnouncement(
      `Ticket ${index + 1} retiré de cette révision. Vous pouvez annuler ce retrait.`,
    );
  }
  function restore() {
    if (
      !undo ||
      undo.submission !== submission ||
      busy ||
      value.tickets.length >= 30
    )
      return;
    const tickets = [...value.tickets],
      nextKeys = [...keys];
    tickets.splice(undo.index, 0, undo.ticket);
    nextKeys.splice(undo.index, 0, undo.key);
    focusNext.current = undo.key;
    setKeys(nextKeys);
    onChange({ ...value, tickets });
    setUndo(null);
    setAnnouncement(
      `Ticket ${undo.index + 1} restauré avec son contenu et ses citations.`,
    );
  }
  const errorProps = (key: string, id: string) => ({
    "aria-invalid": Boolean(errors[key]),
    "aria-describedby": errors[key] ? `${id}-error` : undefined,
  });
  const message = (key: string, id: string) =>
    errors[key] ? (
      <p id={`${id}-error`} className="text-sm text-destructive">
        {errors[key]}
      </p>
    ) : null;
  return (
    <fieldset className="min-w-0 space-y-5" disabled={busy}>
      <legend className="mb-3 text-sm font-semibold">
        Relire les sections et les tickets
      </legend>
      <label className="block space-y-2 text-sm">
        Résumé
        <Textarea
          value={value.summary}
          onChange={(e) => onChange({ ...value, summary: e.target.value })}
          maxLength={4000}
          required
          {...errorProps("summary", "draft-summary")}
        />
        {message("summary", "draft-summary")}
      </label>
      {value.sections.map((section, index) => (
        <label
          key={section.key}
          className="block space-y-2 text-sm font-medium"
        >
          {sectionLabel(section.key, section.title)}
          <Textarea
            rows={5}
            value={section.body}
            maxLength={12000}
            required
            {...errorProps(`section.${index}`, `draft-section-${index}`)}
            onChange={(e) =>
              onChange({
                ...value,
                sections: value.sections.map((item, i) =>
                  i === index ? { ...item, body: e.target.value } : item,
                ),
              })
            }
          />
          {message(`section.${index}`, `draft-section-${index}`)}
        </label>
      ))}
      {canChangeCount ? (
        <div className="space-y-2 text-sm">
          <p>
            {value.tickets.length} ticket(s) dans cette révision · de 1 à 30
          </p>
          <p className="text-muted-foreground">
            Retirer un ticket ici ne supprime aucune issue dans vos outils. Les
            numéros de cette nouvelle version peuvent changer ; les anciennes
            versions et publications restent conservées.
          </p>
          <p role="status" aria-live="polite">
            {announcement}
          </p>
          {undo?.submission === submission ? (
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={restore}
            >
              Annuler le retrait
            </Button>
          ) : null}
        </div>
      ) : null}
      {value.tickets.map((ticket, index) => {
        const key = keys[index],
          titleId = `ticket-${key}-title`,
          descriptionId = `ticket-${key}-description`,
          criteriaId = `ticket-${key}-criteria`;
        return (
          <section
            className="min-w-0 space-y-3 rounded-md border p-4"
            key={key}
            aria-label={`Édition du ticket ${index + 1}`}
          >
            <label className="block space-y-2 text-sm" htmlFor={titleId}>
              Titre du ticket {index + 1}
            </label>
            <Input
              id={titleId}
              ref={(element) => {
                if (element) titleInputs.current.set(key, element);
                else titleInputs.current.delete(key);
              }}
              value={ticket.title}
              maxLength={200}
              required
              {...errorProps(`ticket.${index}.title`, titleId)}
              onChange={(e) =>
                onChange({
                  ...value,
                  tickets: value.tickets.map((item, i) =>
                    i === index ? { ...item, title: e.target.value } : item,
                  ),
                })
              }
            />
            {message(`ticket.${index}.title`, titleId)}
            <label className="block space-y-2 text-sm" htmlFor={descriptionId}>
              Description du ticket {index + 1}
            </label>
            <Textarea
              id={descriptionId}
              value={ticket.description}
              maxLength={8000}
              required
              {...errorProps(`ticket.${index}.description`, descriptionId)}
              onChange={(e) =>
                onChange({
                  ...value,
                  tickets: value.tickets.map((item, i) =>
                    i === index
                      ? { ...item, description: e.target.value }
                      : item,
                  ),
                })
              }
            />
            {message(`ticket.${index}.description`, descriptionId)}
            <label className="block space-y-2 text-sm" htmlFor={criteriaId}>
              Critères d’acceptation du ticket {index + 1} · un par ligne
            </label>
            <Textarea
              id={criteriaId}
              value={ticket.acceptance_criteria.join("\n")}
              required
              {...errorProps(`ticket.${index}.criteria`, criteriaId)}
              onChange={(e) =>
                onChange({
                  ...value,
                  tickets: value.tickets.map((item, i) =>
                    i === index
                      ? {
                          ...item,
                          acceptance_criteria: e.target.value.split("\n"),
                        }
                      : item,
                  ),
                })
              }
            />
            {message(`ticket.${index}.criteria`, criteriaId)}
            {!ticket.source_ids.length ? (
              <p className="text-xs text-muted-foreground">
                Aucune citation attribuée à ce ticket. Les sources du document
                restent consultables.
              </p>
            ) : (
              <p className="text-xs text-muted-foreground">
                {ticket.source_ids.length} citation(s) conservée(s) pour ce
                ticket.
              </p>
            )}
            {canChangeCount ? (
              <Button
                type="button"
                variant="outline"
                disabled={busy || value.tickets.length <= 1}
                onClick={() => remove(index)}
              >
                Retirer le ticket {index + 1}
              </Button>
            ) : null}
          </section>
        );
      })}
      {canChangeCount ? (
        <div className="space-y-2">
          <Button
            type="button"
            variant="outline"
            disabled={busy || value.tickets.length >= 30}
            onClick={add}
          >
            Ajouter un ticket
          </Button>
          <p className="text-xs text-muted-foreground">
            {value.tickets.length >= 30
              ? "La limite de 30 tickets est atteinte."
              : "Un ajout est une saisie manuelle, sans citation copiée d’un autre ticket."}
          </p>
          {value.tickets.length === 1 ? (
            <p className="text-xs text-muted-foreground">
              Conservez au moins un ticket dans ce livrable.
            </p>
          ) : null}
        </div>
      ) : null}
      <label className="block space-y-2 text-sm">
        Points à clarifier · un par ligne
        <Textarea
          value={value.open_questions.join("\n")}
          {...errorProps("questions", "draft-questions")}
          onChange={(e) =>
            onChange({ ...value, open_questions: e.target.value.split("\n") })
          }
        />
        {message("questions", "draft-questions")}
      </label>
      <p className="text-xs text-muted-foreground">
        Les sections structurées et le document envoyé à vos outils restent
        identiques. Les citations des entrées conservées et les sources de cette
        version restent inchangées.
      </p>
    </fieldset>
  );
}
