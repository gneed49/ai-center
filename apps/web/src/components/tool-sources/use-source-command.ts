import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { captureRequestContext } from "@/api/request-context";
import { toolSourcesApi } from "@/api/tool-sources";
import type {
  SourceAction,
  SourceCommandReceipt,
} from "@/api/tool-source-types";
import {
  newSourceCommand,
  readSourceCommand,
  saveSourceCommand,
  sourceCommandKey,
  type SavedSourceCommand,
  type SourceCommandInput,
} from "./source-command";
export function useSourceCommand(
  projectId: string,
  action: SourceAction,
  referenceId: string | null,
) {
  const storageKey = sourceCommandKey(projectId, action, referenceId);
  const [saved, setSaved] = useState(() =>
    readSourceCommand(storageKey, projectId, action, referenceId),
  );
  const commandRef = useRef(saved);
  const [storageError, setStorageError] = useState<Error | null>(null);
  const cache = useQueryClient();
  const queryKey = ["tool-source-command", projectId, action, saved?.key];
  const mutation = useMutation({
    mutationFn: async (command: SavedSourceCommand) => {
      const context = captureRequestContext();
      const value =
        command.action === "attach"
          ? await toolSourcesApi.attach(projectId, command.input, command.key)
          : command.action === "rebind"
            ? await toolSourcesApi.rebind(
                command.referenceId,
                command.input,
                command.key,
              )
            : await toolSourcesApi[command.action](
                command.referenceId,
                command.input,
                command.key,
              );
      context.assertCurrent();
      if (
        value.action !== command.action ||
        value.reference.project_id !== projectId ||
        (command.referenceId &&
          value.reference.public_id !== command.referenceId)
      )
        throw new Error("Le résultat reçu ne correspond pas à cette demande.");
      return { value, context };
    },
    retry: false,
    onSuccess: ({ value, context }, command) => {
      context.assertCurrent();
      cache.setQueryData<SourceCommandReceipt>(
        ["tool-source-command", projectId, action, command.key],
        {
          status: "completed",
          can_retry: false,
          retry_after: null,
          result: value,
          error: null,
        },
      );
    },
    onSettled: (_data, _error, command) =>
      void cache.invalidateQueries({
        queryKey: ["tool-source-command", projectId, action, command.key],
      }),
  });
  const receipt = useQuery({
    queryKey,
    // Wait until the in-flight POST settles before reading its receipt. A
    // premature not_received response must not win over the completed command.
    enabled: Boolean(saved) && !mutation.isPending,
    queryFn: async () => {
      const result = await toolSourcesApi.receipt(
        projectId,
        action,
        saved!.key,
      );
      if (
        result.result &&
        (result.result.action !== action ||
          result.result.reference.project_id !== projectId ||
          (referenceId && result.result.reference.public_id !== referenceId))
      )
        throw new Error("Le reçu ne correspond pas à la demande enregistrée.");
      return result;
    },
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.status === "processing" && !query.state.error
        ? 2000
        : false,
  });
  const appliedReceipt = useRef<string | null>(null);
  const completedReference =
    receipt.data?.status === "completed"
      ? receipt.data.result?.reference.public_id
      : undefined;
  const completedKey = saved?.key;
  useEffect(() => {
    if (
      !completedReference ||
      !completedKey ||
      appliedReceipt.current === completedKey
    )
      return;
    appliedReceipt.current = completedKey;
    // A response recovered by GET has the same effect on local views as a POST
    // response. These invalidations only reload persisted state.
    for (const key of [
      ["tool-sources", projectId],
      ["tool-source", completedReference],
      ["tool-source-history", completedReference],
      ["source-observation"],
      ["work-tools"],
    ])
      void cache.invalidateQueries({ queryKey: key });
  }, [cache, completedKey, completedReference, projectId]);
  function submit(value: SourceCommandInput) {
    if (
      commandRef.current ||
      mutation.isPending ||
      value.action !== action ||
      value.referenceId !== referenceId
    )
      return;
    const existing = readSourceCommand(
      storageKey,
      projectId,
      action,
      referenceId,
    );
    if (existing) {
      commandRef.current = existing;
      setSaved(existing);
      return;
    }
    const command = newSourceCommand(projectId, value);
    try {
      saveSourceCommand(storageKey, command);
    } catch {
      setStorageError(
        new Error(
          "Impossible de conserver le suivi de cette demande sur cet appareil. Aucune lecture n’a été lancée.",
        ),
      );
      return;
    }
    commandRef.current = command;
    setSaved(command);
    setStorageError(null);
    mutation.mutate(command);
  }
  function retry() {
    if (
      saved &&
      !receipt.error &&
      receipt.data?.can_retry &&
      !mutation.isPending &&
      receipt.data.status !== "processing"
    )
      mutation.mutate(saved);
  }
  function clear() {
    if (
      receipt.error ||
      !receipt.data ||
      !["completed", "failed", "expired"].includes(receipt.data.status) ||
      mutation.isPending
    )
      return;
    try {
      localStorage.removeItem(storageKey);
    } catch {
      setStorageError(
        new Error("Impossible d’effacer le suivi local de cette demande."),
      );
      return;
    }
    commandRef.current = null;
    setSaved(null);
    setStorageError(null);
    mutation.reset();
  }
  return {
    saved,
    submit,
    retry,
    clear,
    receipt,
    busy: mutation.isPending,
    error: storageError ?? mutation.error,
  };
}
export type SourceCommandController = ReturnType<typeof useSourceCommand>;
