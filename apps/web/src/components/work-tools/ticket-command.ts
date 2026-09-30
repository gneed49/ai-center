import { createIdempotencyKey } from "@/api/client";
import { draftStorageIdentity } from "@/api/request-context";
import type { PublishTickets } from "@/api/ticket-publication-types";
export interface TicketCommand {
  schema: 1;
  key: string;
  createdAt: number;
  sourceVersionNumber: number;
  input: PublishTickets;
}
export function ticketCommandKey(artifact: string, project?: string) {
  const identity = draftStorageIdentity();
  // Version/destination stay in the payload: moving to v2 must still recover v1.
  return `ai-center.ticket-publication:${identity.actorId}:${identity.workspaceId}:${project ?? "company"}:${artifact}`;
}
export function readTicketCommand(key: string): TicketCommand | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw || raw.length > 8000) return null;
    const value = JSON.parse(raw) as TicketCommand;
    const input = value.input;
    if (
      value.schema !== 1 ||
      !/^[a-f0-9-]{36}$/i.test(value.key) ||
      !Number.isFinite(value.createdAt) ||
      !Number.isInteger(value.sourceVersionNumber) ||
      value.sourceVersionNumber < 1 ||
      !input ||
      ![input.version_id, input.connection_id, input.expected_target_id].every(
        (s) => typeof s === "string" && s.length > 0 && s.length <= 256,
      ) ||
      !["linear", "github"].includes(input.expected_provider) ||
      !Array.isArray(input.ticket_indexes) ||
      input.ticket_indexes.length < 1 ||
      input.ticket_indexes.length > 30 ||
      input.ticket_indexes.some(
        (n) => !Number.isInteger(n) || n < 0 || n > 29,
      ) ||
      new Set(input.ticket_indexes).size !== input.ticket_indexes.length ||
      ![input.preview_fingerprint, input.prior_publications_fingerprint].every(
        (s) => typeof s === "string" && /^[a-f0-9]{64}$/i.test(s),
      ) ||
      typeof input.confirm_additional_issues !== "boolean"
    )
      return null;
    return value;
  } catch {
    return null;
  }
}
export function saveTicketCommand(key: string, command: TicketCommand) {
  // Opaque command IDs, hashes and selection only; no provider/Auth secret or body.
  localStorage.setItem(key, JSON.stringify(command));
}

export function newTicketCommand(
  input: PublishTickets,
  sourceVersionNumber: number,
): TicketCommand {
  return {
    schema: 1,
    key: createIdempotencyKey(),
    createdAt: Date.now(),
    sourceVersionNumber,
    input,
  };
}
