import { notifyRequestError } from "@/lib/request-error";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, ArrowRight, Check, Network } from "lucide-react";
import { useState } from "react";
import type { FormEvent } from "react";
import { Link, useNavigate } from "react-router";
import { toast } from "sonner";

import { api, createIdempotencyKey } from "@/api/client";
import { ErrorState, PageHeader } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

type CreateProjectCommand = Readonly<{
  input: Readonly<{
    name: string;
    objective: string;
  }>;
  idempotencyKey: string;
}>;

export function NewProjectPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [objective, setObjective] = useState("");
  const create = useMutation({
    mutationFn: (command: CreateProjectCommand) =>
      api.createProject(command.input, command.idempotencyKey),
    onSuccess: (project) => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      toast.success("Projet et scopes créés");
      navigate(`/projects/${project.public_id}`);
    },
    onError: notifyRequestError,
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || create.isPending) return;
    create.mutate({
      input: { name, objective },
      idempotencyKey: createIdempotencyKey(),
    });
  }

  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="Nouveau projet"
        title="Donnez une intention au control plane."
        description="Le modèle Software Product Delivery instancie deux scopes spécialisés, leurs contrats et un chemin de handoff explicite."
        actions={
          <Button variant="outline" asChild>
            <Link to="/">
              <ArrowLeft />
              Center
            </Link>
          </Button>
        }
      />
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_380px]">
        <form
          onSubmit={submit}
          className="border border-slate-200 bg-white p-6 sm:p-8"
          aria-busy={create.isPending}
        >
          <div className="max-w-2xl space-y-7">
            <Field>
              <FieldLabel htmlFor="project-name">Nom du projet</FieldLabel>
              <Input
                id="project-name"
                value={name}
                onChange={(event) => {
                  if (create.isError) create.reset();
                  setName(event.target.value);
                }}
                placeholder="Portail partenaires"
                autoFocus
                required
                disabled={create.isPending}
              />
              <FieldDescription>
                Un nom court, stable et visible dans le Center.
              </FieldDescription>
            </Field>
            <Field>
              <FieldLabel htmlFor="project-objective">
                Objectif initial
              </FieldLabel>
              <Textarea
                id="project-objective"
                value={objective}
                onChange={(event) => {
                  if (create.isError) create.reset();
                  setObjective(event.target.value);
                }}
                placeholder="Réduire le temps de traitement d’une demande partenaire tout en conservant une décision traçable."
                rows={6}
                disabled={create.isPending}
              />
              <FieldDescription>
                L’agent Produit le challengera avant toute confirmation dans le
                graphe.
              </FieldDescription>
            </Field>
            {create.error ? (
              <ErrorState
                error={create.error}
                title="Le projet n’a pas été créé"
                retry={() => {
                  if (create.variables) create.mutate(create.variables);
                }}
              />
            ) : null}
            <div className="flex items-center justify-end gap-3 border-t border-slate-100 pt-6">
              <Button variant="ghost" asChild>
                <Link to="/">Annuler</Link>
              </Button>
              <Button type="submit" disabled={!name.trim() || create.isPending}>
                {create.isPending ? "Création…" : "Créer les scopes"}
                <ArrowRight />
              </Button>
            </div>
          </div>
        </form>
        <aside className="border border-slate-200 bg-[#11182b] p-6 text-white sm:p-7">
          <div className="flex items-center gap-3">
            <span className="grid size-10 place-items-center border border-indigo-300/20 bg-indigo-400/10">
              <Network className="size-5 text-indigo-300" />
            </span>
            <div>
              <p className="text-sm font-semibold">Software Product Delivery</p>
              <p className="text-xs text-slate-400">Template système · v1</p>
            </div>
          </div>
          <div className="mt-8 space-y-5">
            <TemplateStep
              number="01"
              title="Scope Produit"
              description="Intention, règles, exigences et critères."
            />
            <TemplateStep
              number="02"
              title="ProductReadyGate"
              description="Manques explicites avant transmission."
            />
            <TemplateStep
              number="03"
              title="Scope Tech"
              description="Plan, preuves et couverture sourcée."
            />
          </div>
          <div className="mt-8 border-t border-white/10 pt-5 text-xs leading-5 text-slate-400">
            <p className="flex gap-2">
              <Check className="mt-0.5 size-3.5 shrink-0 text-emerald-400" />
              Chaque vérité reste proposée jusqu’à votre confirmation.
            </p>
          </div>
        </aside>
      </div>
    </div>
  );
}

function TemplateStep({
  number,
  title,
  description,
}: {
  number: string;
  title: string;
  description: string;
}) {
  return (
    <div className="grid grid-cols-[32px_1fr] gap-3">
      <span className="font-mono text-xs text-indigo-300">{number}</span>
      <div>
        <p className="text-sm font-medium">{title}</p>
        <p className="mt-1 text-xs leading-5 text-slate-400">{description}</p>
      </div>
    </div>
  );
}
