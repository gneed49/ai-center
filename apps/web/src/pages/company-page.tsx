import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState, type FormEvent } from "react";
import { Link } from "react-router";
import { Building2, Network } from "lucide-react";
import { companyApi } from "@/api/company";
import type { CompanyOverview } from "@/api/company-types";
import { createIdempotencyKey } from "@/api/client";
import { ErrorState, LoadingState, PageHeader } from "@/components/app/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { AutomationControls } from "@/components/company/automation-controls";
import { CompanyDataControls } from "@/components/company/data-controls";
import { TeamAccess } from "@/components/team/team-access";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";

export function CompanyPage() {
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  if (company.isPending)
    return <LoadingState label="Chargement de votre entreprise…" />;
  if (company.error)
    return (
      <ErrorState error={company.error} retry={() => void company.refetch()} />
    );
  return (
    <div className="space-y-8">
      <PageHeader
        eyebrow="L’espace commun"
        title={
          company.data.setup_complete
            ? company.data.workspace.name
            : "Configurons votre entreprise"
        }
        description="Votre équipe travaille dans les mêmes projets et partage un contexte commun."
        actions={
          <Button variant="outline" asChild>
            <Link to="/graph">
              <Network data-icon="inline-start" />
              Voir le graphe
            </Link>
          </Button>
        }
      />
      <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <CompanySettings company={company.data} />
        <aside className="rounded-xl bg-muted/50 p-6">
          <Building2 aria-hidden className="mb-4 size-6 text-primary" />
          <h2 className="font-semibold">Ce qui vous rassemble</h2>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            L’espace général conserve les règles et les repères de l’entreprise.
            Chaque projet garde ses conversations, ses décisions et ses
            livrables.
          </p>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            Les membres ont accès à l’ensemble des projets de cette entreprise,
            selon leur rôle. Les autres entreprises restent séparées.
          </p>
          <Button variant="link" className="mt-4 px-0" asChild>
            <Link to="/agents">Échanger avec un agent</Link>
          </Button>
        </aside>
      </div>
      <TeamAccess role={company.data.workspace.role} />
      <AutomationControls owner={company.data.workspace.role === "owner"} />
      {company.data.workspace.role === "owner" ? (
        <CompanyDataControls
          workspaceId={company.data.workspace.public_id}
          projects={company.data.projects}
        />
      ) : null}
    </div>
  );
}

function CompanySettings({ company }: { company: CompanyOverview }) {
  const [name, setName] = useState(company.workspace.name);
  const [description, setDescription] = useState(company.workspace.description);
  const [saved, setSaved] = useState(false);
  const pending = useRef<{ fingerprint: string; key: string } | null>(null);
  const cache = useQueryClient();
  const owner = company.workspace.role === "owner";
  const save = useMutation({
    mutationFn: () => {
      const input = { name: name.trim(), description: description.trim() };
      const fingerprint = JSON.stringify(input);
      if (pending.current?.fingerprint !== fingerprint)
        pending.current = { fingerprint, key: createIdempotencyKey() };
      return (company.setup_complete ? companyApi.update : companyApi.setup)(
        input,
        pending.current.key,
      );
    },
    onSuccess: async (data) => {
      pending.current = null;
      setSaved(true);
      cache.setQueryData(["company"], data);
      await cache.invalidateQueries({ queryKey: ["graph"] });
      await cache.invalidateQueries({ queryKey: ["workspaces"] });
    },
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    setSaved(false);
    if (name.trim() && owner) save.mutate();
  }
  return (
    <section>
      <h2 className="text-lg font-semibold">
        Le point de départ de votre équipe
      </h2>
      <form onSubmit={submit} className="mt-5 space-y-5">
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="company-name">Nom de l’entreprise</FieldLabel>
            <Input
              id="company-name"
              value={name}
              onChange={(event) => {
                setName(event.target.value);
                setSaved(false);
              }}
              maxLength={120}
              required
              disabled={!owner || save.isPending}
              autoComplete="organization"
            />
          </Field>
          <Field>
            <FieldLabel htmlFor="company-description">
              Votre activité et votre mission
            </FieldLabel>
            <Textarea
              id="company-description"
              value={description}
              onChange={(event) => {
                setDescription(event.target.value);
                setSaved(false);
              }}
              maxLength={4000}
              rows={5}
              disabled={!owner || save.isPending}
              placeholder="Ce que votre entreprise veut accomplir et pour qui."
            />
            <FieldDescription>
              Présentez l’entreprise avec les mots de votre équipe.
            </FieldDescription>
          </Field>
        </FieldGroup>
        {owner ? (
          <Button type="submit" disabled={save.isPending || !name.trim()}>
            {save.isPending
              ? "Enregistrement…"
              : company.setup_complete
                ? "Enregistrer"
                : "Préparer l’espace de l’entreprise"}
          </Button>
        ) : (
          <p className="text-sm text-muted-foreground">
            Le propriétaire de l’entreprise peut modifier ces informations.
          </p>
        )}
        {save.error && (
          <p role="alert" className="text-sm text-destructive">
            {save.error.message}
          </p>
        )}
        {saved && (
          <p role="status" className="text-sm text-teal-700">
            Les informations ont été enregistrées.
          </p>
        )}
      </form>
    </section>
  );
}
