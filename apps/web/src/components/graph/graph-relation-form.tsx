import { useRef, useState, type FormEvent } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link2 } from "lucide-react";
import type { GraphNode } from "@/api/company-types";
import { companyApi } from "@/api/company";
import { createIdempotencyKey } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { graphKindLabels, nodeKey } from "@/lib/graph-layout";

export function GraphRelationForm({
  source,
  nodes,
}: {
  source: GraphNode;
  nodes: GraphNode[];
}) {
  const [targetKey, setTargetKey] = useState("");
  const [relation, setRelation] = useState("references");
  const [success, setSuccess] = useState(false);
  const pendingCommand = useRef<{
    fingerprint: string;
    publicId: string;
    key: string;
  } | null>(null);
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: async () => {
      const target = nodes.find((node) => nodeKey(node) === targetKey);
      if (!target) throw new Error("Choisissez un élément à relier.");
      const fingerprint = JSON.stringify([
        nodeKey(source),
        targetKey,
        relation,
      ]);
      if (pendingCommand.current?.fingerprint !== fingerprint)
        pendingCommand.current = {
          fingerprint,
          publicId: crypto.randomUUID(),
          key: createIdempotencyKey(),
        };
      const command = pendingCommand.current;
      return companyApi.createEdge(
        {
          public_id: command.publicId,
          source: {
            kind: source.kind,
            public_id: source.id,
            project_public_id: source.project_public_id,
          },
          target: {
            kind: target.kind,
            public_id: target.id,
            project_public_id: target.project_public_id,
          },
          edge_type: relation,
        },
        command.key,
      );
    },
    onSuccess: async () => {
      pendingCommand.current = null;
      setSuccess(true);
      setTargetKey("");
      await queryClient.invalidateQueries({ queryKey: ["graph"] });
    },
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    setSuccess(false);
    mutation.mutate();
  }
  const targets = nodes.filter((node) => nodeKey(node) !== nodeKey(source));
  if (!targets.length) return null;
  return (
    <details className="mt-6 border-t border-border pt-4">
      <summary className="cursor-pointer text-sm font-medium text-primary">
        Relier à un autre élément
      </summary>
      <form className="mt-4" onSubmit={submit}>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="graph-target">Élément à relier</FieldLabel>
            <Select
              value={targetKey}
              onValueChange={setTargetKey}
              disabled={mutation.isPending}
            >
              <SelectTrigger id="graph-target" className="w-full">
                <SelectValue placeholder="Choisir un élément" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {targets.map((node) => (
                    <SelectItem key={nodeKey(node)} value={nodeKey(node)}>
                      {node.label}
                      {node.version_number
                        ? ` · v${node.version_number}`
                        : ""}{" "}
                      · {graphKindLabels[node.kind]}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field>
            <FieldLabel htmlFor="graph-relation">Nature du lien</FieldLabel>
            <Select
              value={relation}
              onValueChange={setRelation}
              disabled={mutation.isPending}
            >
              <SelectTrigger id="graph-relation" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="references">Référence</SelectItem>
                  <SelectItem value="depends_on">Dépend de</SelectItem>
                  <SelectItem value="informs">Éclaire</SelectItem>
                  <SelectItem value="derived_from">Provient de</SelectItem>
                  <SelectItem value="contradicts">Contredit</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Button
            type="submit"
            size="sm"
            disabled={!targetKey || mutation.isPending}
          >
            <Link2 data-icon="inline-start" />
            {mutation.isPending ? "Enregistrement…" : "Enregistrer le lien"}
          </Button>
        </FieldGroup>
        {mutation.error && (
          <p role="alert" className="mt-3 text-sm text-destructive">
            {mutation.error.message}
          </p>
        )}
        {success && (
          <p role="status" className="mt-3 text-sm text-teal-700">
            Le lien a été enregistré.
          </p>
        )}
      </form>
    </details>
  );
}
