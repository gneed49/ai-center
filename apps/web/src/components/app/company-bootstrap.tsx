import { useRef, useState, type FormEvent } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Building2 } from "lucide-react";
import { companyApi } from "@/api/company";
import { createIdempotencyKey } from "@/api/client";
import type { WorkspaceSummary } from "@/api/types";
import { ErrorState, LoadingState } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";

export function CompanyBootstrap({
  onCreated,
}: {
  onCreated: (workspace: WorkspaceSummary) => void;
}) {
  const capabilities = useQuery({
    queryKey: ["workspace-capabilities"],
    queryFn: companyApi.capabilities,
  });
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const command = useRef<{
    fingerprint: string;
    id: string;
    key: string;
  } | null>(null);
  const create = useMutation({
    mutationFn: () => {
      const input = { name: name.trim(), description: description.trim() };
      const fingerprint = JSON.stringify(input);
      if (command.current?.fingerprint !== fingerprint)
        command.current = {
          fingerprint,
          id: crypto.randomUUID(),
          key: createIdempotencyKey(),
        };
      return companyApi.create(
        { ...input, public_id: command.current.id },
        command.current.key,
      );
    },
    onSuccess: onCreated,
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    if (name.trim() && capabilities.data?.can_create_company) create.mutate();
  }
  if (capabilities.isPending)
    return <LoadingState label="Vérification de votre accès…" />;
  if (capabilities.isError)
    return (
      <ErrorState
        error={capabilities.error}
        retry={() => void capabilities.refetch()}
      />
    );
  if (!capabilities.data.can_create_company)
    return (
      <section className="space-y-3">
        <Building2 className="size-6 text-primary" aria-hidden />
        <h2 className="text-xl font-semibold">Rejoindre votre entreprise</h2>
        <p className="text-sm leading-6 text-muted-foreground">
          Demandez une invitation à la personne qui gère votre entreprise, puis
          ouvrez son lien pour rejoindre l’équipe.
        </p>
      </section>
    );
  return (
    <form onSubmit={submit} className="space-y-5">
      <div>
        <Building2 className="mb-3 size-6 text-primary" aria-hidden />
        <h2 className="text-xl font-semibold">
          Votre entreprise, au même endroit
        </h2>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">
          Créez son espace, puis rassemblez votre équipe et vos projets.
        </p>
      </div>
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="new-company-name">
            Nom de l’entreprise
          </FieldLabel>
          <Input
            id="new-company-name"
            required
            autoComplete="organization"
            maxLength={120}
            value={name}
            onChange={(event) => setName(event.target.value)}
            disabled={create.isPending}
            placeholder="Le nom connu de votre équipe"
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="new-company-description">
            Votre activité en quelques mots
          </FieldLabel>
          <Textarea
            id="new-company-description"
            maxLength={4000}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            disabled={create.isPending}
            placeholder="Ce que vous construisez, pour qui et dans quel but."
          />
          <FieldDescription>
            Facultatif. Vous pourrez préciser ce contexte au fil du travail.
          </FieldDescription>
        </Field>
      </FieldGroup>
      {create.error && (
        <p role="alert" className="text-sm text-destructive">
          La création n’a pas été confirmée. Réessayez avec les mêmes
          informations pour reprendre la demande. {create.error.message}
        </p>
      )}
      <Button type="submit" disabled={create.isPending || !name.trim()}>
        {create.isPending
          ? "Création de votre espace…"
          : "Créer l’espace de mon entreprise"}
      </Button>
    </form>
  );
}
