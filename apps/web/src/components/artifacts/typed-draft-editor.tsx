import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { sectionLabel, type TypedDraft } from "./typed-draft";
export function TypedDraftEditor({
  value,
  onChange,
  busy,
}: {
  value: TypedDraft;
  onChange: (value: TypedDraft) => void;
  busy: boolean;
}) {
  return (
    <fieldset className="space-y-5" disabled={busy}>
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
        />
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
            onChange={(e) =>
              onChange({
                ...value,
                sections: value.sections.map((item, i) =>
                  i === index ? { ...item, body: e.target.value } : item,
                ),
              })
            }
          />
        </label>
      ))}
      {value.tickets.map((ticket, index) => (
        <div className="space-y-3 rounded-md border p-4" key={index}>
          <label className="block space-y-2 text-sm">
            Titre du ticket {index + 1}
            <Input
              value={ticket.title}
              maxLength={200}
              required
              onChange={(e) =>
                onChange({
                  ...value,
                  tickets: value.tickets.map((item, i) =>
                    i === index ? { ...item, title: e.target.value } : item,
                  ),
                })
              }
            />
          </label>
          <label className="block space-y-2 text-sm">
            Description du ticket {index + 1}
            <Textarea
              value={ticket.description}
              maxLength={8000}
              required
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
          </label>
          <label className="block space-y-2 text-sm">
            Critères d’acceptation du ticket {index + 1} · un par ligne
            <Textarea
              value={ticket.acceptance_criteria.join("\n")}
              required
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
          </label>
        </div>
      ))}
      <label className="block space-y-2 text-sm">
        Points à clarifier · un par ligne
        <Textarea
          value={value.open_questions.join("\n")}
          onChange={(e) =>
            onChange({
              ...value,
              open_questions: e.target.value.split("\n").filter(Boolean),
            })
          }
        />
      </label>
      <p className="text-xs text-muted-foreground">
        Les sections structurées et le document envoyé à vos outils restent
        identiques. Les sources citées par l’agent sont conservées.
      </p>
    </fieldset>
  );
}
