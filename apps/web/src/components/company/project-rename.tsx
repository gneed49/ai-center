import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, ApiError, createIdempotencyKey } from "@/api/client";
import { companyApi } from "@/api/company";
import { companyControlsApi } from "@/api/company-controls";
import type { ProjectSummary } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorState } from "@/components/app/page";

export function ProjectRename({ project }: { project: ProjectSummary }) {
  const cache = useQueryClient();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const [base, setBase] = useState<ProjectSummary | null>(null);
  const [name, setName] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const command = useRef<{ fingerprint: string; key: string } | null>(null);
  const rename = useMutation({
    mutationFn: (input: { name: string; expected_updated_at: string }) => {
      const fingerprint = JSON.stringify([project.public_id, input]);
      if (command.current?.fingerprint !== fingerprint)
        command.current = { fingerprint, key: createIdempotencyKey() };
      return companyControlsApi.renameProject(
        project.public_id,
        input,
        command.current.key,
      );
    },
    onSuccess: async () => {
      setBase(null);
      command.current = null;
      setNotice("Nom du projet enregistré.");
      await Promise.all(
        [
          ["snapshot", project.public_id],
          ["projects"],
          ["company"],
          ["graph"],
        ].map((queryKey) => cache.invalidateQueries({ queryKey })),
      );
    },
  });
  const reload = useMutation({
    mutationFn: () => api.snapshot(project.public_id),
    onSuccess: (result) => {
      setBase(result.project);
      setName(result.project.name);
      rename.reset();
    },
  });
  const canRename =
    company.data?.workspace.role !== "viewer" &&
    Boolean(company.data) &&
    project.status === "active";
  if (!canRename) return null;
  return (
    <section className="space-y-3" aria-label="Nom du projet">
      {base ? (
        <form
          className="space-y-3 rounded-md border p-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (name.trim() && name.trim().length <= 120)
              rename.mutate({
                name: name.trim(),
                expected_updated_at: base.updated_at,
              });
          }}
        >
          <label
            htmlFor="project-rename-name"
            className="block text-sm font-medium"
          >
            Nouveau nom du projet
          </label>
          <Input
            id="project-rename-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            maxLength={120}
            disabled={rename.isPending || reload.isPending}
          />
          <div className="flex flex-wrap gap-2">
            <Button
              type="submit"
              disabled={!name.trim() || rename.isPending || reload.isPending}
            >
              Enregistrer le nom
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={rename.isPending}
              onClick={() => {
                setBase(null);
                rename.reset();
              }}
            >
              Annuler
            </Button>
          </div>
          {rename.error ? (
            <ErrorState
              title="Le renommage n’a pas pu être confirmé"
              error={rename.error}
            />
          ) : null}
          {rename.error instanceof ApiError && rename.error.status === 409 ? (
            <Button
              type="button"
              variant="outline"
              disabled={reload.isPending}
              onClick={() => reload.mutate()}
            >
              Charger le nom actuel
            </Button>
          ) : null}
          {reload.error ? <ErrorState error={reload.error} /> : null}
        </form>
      ) : (
        <Button
          variant="outline"
          onClick={() => {
            setBase(project);
            setName(project.name);
            setNotice(null);
            rename.reset();
          }}
        >
          Renommer le projet
        </Button>
      )}
      {notice ? (
        <p role="status" className="text-sm text-emerald-800">
          {notice}
        </p>
      ) : null}
    </section>
  );
}
