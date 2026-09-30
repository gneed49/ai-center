import { request } from "./client";
import type {
  AttachToolSource,
  ExpectedSource,
  ObservationKind,
  RebindToolSource,
  SourceAction,
  SourceCommandReceipt,
  SourceCommandResult,
  SourceObservation,
  SourceObservationDetail,
  SourcePage,
  ToolSourceDetail,
  ToolSourceList,
} from "./tool-source-types";
const id = encodeURIComponent;
export function observationPath(kind: string, publicId: string): string | null {
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(
      publicId,
    )
  )
    return null;
  if (kind === "tool_source_observation")
    return `/source-observations/${publicId}`;
  if (kind === "publication_observation")
    return `/publication-observations/${publicId}`;
  return null;
}
function validateObservation(
  value: SourceObservation,
  kind: ObservationKind,
  publicId: string,
) {
  const imported = kind === "tool_source_observation";
  if (
    value.public_id !== publicId ||
    value.source_kind !== kind ||
    value.trust !== "observed_external" ||
    value.mandatory !== false ||
    (value.external_id === null &&
      (value.freshness.eligible ||
        value.freshness.is_current ||
        !value.freshness.reasons.includes("invalid_identity"))) ||
    (imported
      ? !value.reference_public_id ||
        value.publication_public_id !== null ||
        value.external_id === null
      : !value.publication_public_id || value.reference_public_id !== null)
  )
    throw new Error("La source reçue ne correspond pas à la lecture demandée.");
}
function validateSource(
  value: ToolSourceDetail,
  referenceId?: string,
  projectId?: string,
) {
  const { reference, observation } = value;
  validateObservation(
    observation,
    "tool_source_observation",
    reference.current_observation_id,
  );
  if (
    (referenceId && reference.public_id !== referenceId) ||
    (projectId && reference.project_id !== projectId) ||
    observation.reference_public_id !== reference.public_id ||
    observation.source_project_public_id !== reference.project_id
  )
    throw new Error("La source reçue n’appartient pas au contexte demandé.");
  return value;
}
function post<T>(path: string, input: unknown, key: string) {
  return request<T>(path, {
    method: "POST",
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  });
}
export const toolSourcesApi = {
  list: async (
    projectId: string,
    filters: { provider?: string; status?: string; cursor?: string } = {},
  ) => {
    const query = new URLSearchParams({ limit: "25" });
    for (const [key, value] of Object.entries(filters))
      if (value) query.set(key, value);
    const data = await request<ToolSourceList>(
      `/api/projects/${id(projectId)}/tool-sources?${query}`,
      { cache: "no-store" },
    );
    data.items.forEach((item) => validateSource(item, undefined, projectId));
    return data;
  },
  source: async (referenceId: string) =>
    validateSource(
      await request<ToolSourceDetail>(`/api/tool-sources/${id(referenceId)}`, {
        cache: "no-store",
      }),
      referenceId,
    ),
  observation: async (kind: ObservationKind, publicId: string) => {
    const route =
      kind === "tool_source_observation"
        ? "tool-source-observations"
        : "publication-observations";
    const value = await request<SourceObservationDetail>(
      `/api/${route}/${id(publicId)}`,
      { cache: "no-store" },
    );
    validateObservation(value.observation, kind, publicId);
    return value;
  },
  history: async (referenceId: string, cursor?: string) => {
    const query = new URLSearchParams({
      limit: "25",
      ...(cursor ? { cursor } : {}),
    });
    const data = await request<SourcePage<SourceObservation>>(
      `/api/tool-sources/${id(referenceId)}/observations?${query}`,
      { cache: "no-store" },
    );
    for (const value of data.items) {
      validateObservation(value, "tool_source_observation", value.public_id);
      if (value.reference_public_id !== referenceId)
        throw new Error("L’historique reçu appartient à une autre source.");
    }
    return data;
  },
  attach: (projectId: string, input: AttachToolSource, key: string) =>
    post<SourceCommandResult>(
      `/api/projects/${id(projectId)}/tool-sources`,
      input,
      key,
    ),
  refresh: (referenceId: string, input: ExpectedSource, key: string) =>
    post<SourceCommandResult>(
      `/api/tool-sources/${id(referenceId)}/refresh`,
      input,
      key,
    ),
  detach: (referenceId: string, input: ExpectedSource, key: string) =>
    post<SourceCommandResult>(
      `/api/tool-sources/${id(referenceId)}/detach`,
      input,
      key,
    ),
  rebind: (referenceId: string, input: RebindToolSource, key: string) =>
    post<SourceCommandResult>(
      `/api/tool-sources/${id(referenceId)}/rebind`,
      input,
      key,
    ),
  receipt: (projectId: string, action: SourceAction, key: string) =>
    request<SourceCommandReceipt>(
      `/api/projects/${id(projectId)}/tool-source-commands/${id(key)}?operation=${action}`,
      { cache: "no-store" },
    ),
};
