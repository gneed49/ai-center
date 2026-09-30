import { createIdempotencyKey } from "@/api/client";
import { draftStorageIdentity } from "@/api/request-context";
import type {
  AttachToolSource,
  ExpectedSource,
  RebindToolSource,
  SourceAction,
  SourceProvider,
} from "@/api/tool-source-types";
export type SourceCommandInput =
  | { action: "attach"; referenceId: null; input: AttachToolSource }
  | { action: "refresh" | "detach"; referenceId: string; input: ExpectedSource }
  | { action: "rebind"; referenceId: string; input: RebindToolSource };
export type SavedSourceCommand = {
  schema: 1;
  key: string;
  createdAt: number;
  projectId: string;
} & SourceCommandInput;
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const nonzeroId = (value: unknown): value is string =>
  typeof value === "string" &&
  uuid.test(value) &&
  value !== "00000000-0000-0000-0000-000000000000";
function canonicalId(value: string) {
  if (
    !/^(?:[0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i.test(
      value,
    )
  )
    return null;
  const simple = value.replaceAll("-", "").toLowerCase();
  if (!/^[0-9a-f]{32}$/.test(simple) || /^0+$/.test(simple)) return null;
  return `${simple.slice(0, 8)}-${simple.slice(8, 12)}-${simple.slice(12, 16)}-${simple.slice(16, 20)}-${simple.slice(20)}`;
}
/** Local allowlist before persistence; the server still validates the locator. */
export function normalizeSourceInput(
  provider: SourceProvider,
  raw: string,
): string {
  const value = raw.trim();
  const invalid = () =>
    new Error(
      "Collez un lien officiel de la page ou du ticket, ou son identifiant. Les liens contenant des accès privés ne sont pas acceptés.",
    );
  if (
    !value ||
    new TextEncoder().encode(value).length > 2048 ||
    Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 || code === 127;
    })
  )
    throw invalid();
  const direct = canonicalId(value);
  if (direct) return direct;
  const linearKey = /^[a-z][a-z0-9]{0,31}-[1-9][0-9]{0,17}$/i;
  if (provider === "linear" && linearKey.test(value))
    return value.toUpperCase();
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw invalid();
  }
  const authority = value.split("://")[1]?.split("/")[0] ?? "";
  const rawPath = value.split("?")[0];
  if (
    url.protocol !== "https:" ||
    /[:@]/.test(authority) ||
    url.username ||
    url.password ||
    url.hash ||
    value.includes("\\") ||
    rawPath.includes("%") ||
    rawPath.split("/").some((part) => part === "." || part === "..")
  )
    throw invalid();
  const parts = url.pathname.replace(/\/$/, "").split("/").slice(1);
  if (provider === "linear") {
    if (
      url.hostname !== "linear.app" ||
      url.search ||
      parts.length < 3 ||
      parts.length > 4 ||
      !/^[a-z0-9-]{1,100}$/i.test(parts[0]) ||
      parts[1] !== "issue" ||
      !linearKey.test(parts[2]) ||
      (parts[3] !== undefined && !/^[a-z0-9_-]+$/i.test(parts[3]))
    )
      throw invalid();
    return `https://linear.app/${parts[0].toLowerCase()}/issue/${parts[2].toUpperCase()}`;
  }
  if (
    !["notion.so", "www.notion.so", "app.notion.com"].includes(url.hostname) ||
    parts.length < 1 ||
    parts.length > 2 ||
    parts.some((part) => !part) ||
    [...url.searchParams].some(
      ([key, val]) => !["pvs", "source"].includes(key) || val.length > 64,
    )
  )
    throw invalid();
  const last = parts.at(-1)!;
  const pageId =
    canonicalId(last) ?? canonicalId(last.slice(last.lastIndexOf("-") + 1));
  if (!pageId) throw invalid();
  const ids = new Set<string>();
  for (const width of [32, 36])
    for (let i = 0; i <= url.pathname.length - width; i++) {
      const candidate = canonicalId(url.pathname.slice(i, i + width));
      if (candidate) ids.add(candidate);
    }
  if (ids.size !== 1 || !ids.has(pageId)) throw invalid();
  return `https://www.notion.so/${pageId.replaceAll("-", "")}`;
}
export function sourceCommandKey(
  projectId: string,
  action: SourceAction,
  referenceId: string | null,
) {
  const identity = draftStorageIdentity();
  return `ai-center.tool-source:${identity.actorId}:${identity.workspaceId}:${projectId}:${action}:${referenceId ?? "new"}`;
}
export function newSourceCommand(
  projectId: string,
  value: SourceCommandInput,
): SavedSourceCommand {
  return {
    schema: 1,
    key: createIdempotencyKey(),
    createdAt: Date.now(),
    projectId,
    ...value,
  };
}
export function saveSourceCommand(
  storageKey: string,
  command: SavedSourceCommand,
) {
  localStorage.setItem(storageKey, JSON.stringify(command));
}
export function readSourceCommand(
  storageKey: string,
  projectId: string,
  action: SourceAction,
  referenceId: string | null,
): SavedSourceCommand | null {
  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw || raw.length > 12000) return null;
    const value = JSON.parse(raw) as SavedSourceCommand;
    if (
      value.schema !== 1 ||
      !nonzeroId(value.key) ||
      !Number.isFinite(value.createdAt) ||
      value.projectId !== projectId ||
      !nonzeroId(value.projectId) ||
      value.action !== action ||
      value.referenceId !== referenceId ||
      !value.input ||
      (action !== "attach" && !nonzeroId(value.referenceId))
    )
      return null;
    const input = value.input;
    if (
      value.action !== "attach" &&
      (!Number.isInteger(value.input.expected_revision) ||
        value.input.expected_revision < 1 ||
        !nonzeroId(value.input.expected_observation_id))
    )
      return null;
    if (value.action === "attach" || value.action === "rebind") {
      const data = value.input;
      if (
        !nonzeroId(data.connection_id) ||
        !Number.isInteger(data.expected_connection_revision) ||
        data.expected_connection_revision < 1 ||
        data.confirm_scope_sharing !== true
      )
        return null;
    }
    if (
      value.action === "attach" &&
      (!["linear", "notion"].includes(value.input.provider) ||
        typeof value.input.source !== "string" ||
        normalizeSourceInput(value.input.provider, value.input.source) !==
          value.input.source)
    )
      return null;
    const keys =
      action === "attach"
        ? [
            "connection_id",
            "expected_connection_revision",
            "provider",
            "source",
            "confirm_scope_sharing",
          ]
        : action === "rebind"
          ? [
              "expected_revision",
              "expected_observation_id",
              "connection_id",
              "expected_connection_revision",
              "confirm_scope_sharing",
            ]
          : ["expected_revision", "expected_observation_id"];
    if (
      Object.keys(input).some((key) => !keys.includes(key)) ||
      Object.keys(value).some(
        (key) =>
          ![
            "schema",
            "key",
            "createdAt",
            "projectId",
            "action",
            "referenceId",
            "input",
          ].includes(key),
      )
    )
      return null;
    return value;
  } catch {
    return null;
  }
}
