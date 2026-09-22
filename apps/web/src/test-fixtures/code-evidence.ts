import type { CodeObservationDetail } from "@/api/code-observations";
/** Synthetic source observation; never a real GitHub read. */
export const fixtureCode: CodeObservationDetail = {
  corpus: {
    public_id: "fixture-corpus",
    project_id: "fixture-project",
    connection_id: "fixture-tool",
    repository: "fiction/example",
    commit_sha: "a".repeat(40),
    commit_verified: true,
    requested_paths: ["src/fixture.ts"],
    observed_at: "2026-09-21T10:00:00Z",
  },
  files: [
    {
      public_id: "fixture-file",
      path: "src/fixture.ts",
      status: "code_read",
      reason_code: null,
      blob_sha: "b".repeat(40),
      content_hash: "c".repeat(64),
      content_text: "// [FICTIF]\nconst sample = 1;\nexport { sample };\n",
      line_count: 3,
      observed_at: "2026-09-21T10:00:00Z",
    },
  ],
};
