import { describe, expect, it } from "vitest";
import type { GraphEdge, GraphNode } from "@/api/company-types";
import {
  endpointKey,
  layoutGraph,
  nodeKey,
  sourceLink,
  internalSourceLink,
  visibleEdges,
} from "./graph-layout";

const rule: GraphNode = {
  id: "rule-version",
  kind: "knowledge",
  project_public_id: "project-a",
  scope_kind: "project",
  label: "[FICTIF] Règle",
  status: "active",
  version_public_id: "rule-version",
  source_url: null,
};
const project: GraphNode = {
  ...rule,
  id: "project-a",
  kind: "scope",
  label: "[FICTIF] Projet",
  version_public_id: null,
};
const edge: GraphEdge = {
  id: "edge",
  source_kind: "knowledge",
  source_public_id: rule.id,
  source_project_public_id: "project-a",
  target_kind: "scope",
  target_public_id: project.id,
  target_project_public_id: "project-a",
  edge_type: "references",
  status: "confirmed",
  provenance: { origin: "human" },
};

describe("knowledge graph projection", () => {
  it("preserves an exact internal artifact version without accepting a redirect", () => {
    const id = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
    const app_path = `/artifacts/${id}?version=${id}`;
    expect(internalSourceLink({ ...rule, app_path })).toBe(app_path);
    for (const invalid of [
      "//outside.invalid",
      "https://outside.invalid",
      "/settings/ai",
      `${app_path}&next=https://outside.invalid`,
    ]) {
      expect(internalSourceLink({ ...rule, app_path: invalid })).toBeNull();
    }
  });
  it("keeps type in identity, rather than conflating a scope and artifact with the same UUID", () => {
    const artifact: GraphNode = { ...project, kind: "artifact" };
    expect(nodeKey(artifact)).not.toBe(nodeKey(project));
    expect(endpointKey(edge, "target")).toBe(nodeKey(project));
    expect(visibleEdges([edge], [rule, artifact])).toEqual([]);
  });
  it("does not invent links when filtering hides a source", () => {
    expect(visibleEdges([edge], [rule, project])).toEqual([edge]);
    expect(visibleEdges([edge], [project])).toEqual([]);
    expect(visibleEdges([], [rule, project])).toEqual([]);
  });
  it("produces stable non-overlapping positions with company scope first without mutating input", () => {
    const company: GraphNode = {
      ...project,
      id: "company",
      project_public_id: "zz-company",
      scope_kind: "company",
    };
    const input = [rule, company, project];
    const first = layoutGraph(input);
    const second = layoutGraph([...input].reverse());
    expect(first).toEqual(second);
    expect(input).toEqual([rule, company, project]);
    expect(first.nodes[0].id).toBe("company");
    expect(new Set(first.nodes.map((node) => `${node.x},${node.y}`)).size).toBe(
      3,
    );
    expect(
      first.nodes.every(
        (node) => node.x + 236 <= first.width && node.y + 64 <= first.height,
      ),
    ).toBe(true);
  });
  it("never turns arbitrary schemes or embedded credentials into source links", () => {
    for (const source_url of [
      "javascript:alert(1)",
      "data:text/html,hello",
      "http://source.example/",
      "https://secret:token@source.example/",
      "not a URL",
    ]) {
      expect(sourceLink({ ...rule, source_url })).toBeNull();
    }
    expect(
      sourceLink({ ...rule, source_url: "https://github.com/example/project" }),
    ).toBe("https://github.com/example/project");
  });
});
