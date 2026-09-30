import { request } from "./client";
export type CodeFileStatus =
  | "code_read"
  | "missing"
  | "inaccessible"
  | "too_large"
  | "binary"
  | "unsupported"
  | "unavailable";
export interface CodeReadInput {
  connection_id: string;
  repository: string;
  commit_sha: string;
  paths: string[];
}
export interface CodeCorpus {
  public_id: string;
  project_id: string;
  connection_id: string;
  repository: string;
  commit_sha: string;
  commit_verified: boolean;
  requested_paths: string[];
  observed_at: string;
}
export interface CodeFileObservation {
  public_id: string;
  path: string;
  status: CodeFileStatus;
  reason_code: string | null;
  blob_sha: string | null;
  content_hash: string | null;
  content_text: string | null;
  line_count: number;
  observed_at: string;
}
export interface CodeObservationDetail {
  corpus: CodeCorpus;
  files: CodeFileObservation[];
}
export const codeObservationsApi = {
  list: (projectId: string, offset = 0) =>
    request<{ items: CodeCorpus[]; limit: number; offset: number }>(
      `/api/projects/${encodeURIComponent(projectId)}/code-observations?limit=25&offset=${offset}`,
    ),
  detail: (id: string) =>
    request<CodeObservationDetail>(
      `/api/code-observations/${encodeURIComponent(id)}`,
    ),
  read: (projectId: string, input: CodeReadInput, key: string) =>
    request<CodeObservationDetail>(
      `/api/projects/${encodeURIComponent(projectId)}/code-observations`,
      {
        method: "POST",
        headers: { "Idempotency-Key": key },
        body: JSON.stringify(input),
      },
    ),
};
