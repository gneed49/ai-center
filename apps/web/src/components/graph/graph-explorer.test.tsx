// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { GraphView } from "@/api/company-types";
import { GraphExplorer } from "./graph-explorer";

const graph: GraphView = {
  workspace_public_id: "company",
  project_public_id: null,
  limits: { max_nodes: 200, max_edges: 400 },
  truncated: false,
  source_graph_versions: [{ project_public_id: "project", graph_version: 2 }],
  nodes: [
    {
      id: "version-1",
      kind: "knowledge",
      project_public_id: "project",
      scope_kind: "project",
      label: "[FICTIF] Livraison en France",
      status: "active",
      version_public_id: "version-1",
      source_url: null,
    },
    {
      id: "spec",
      kind: "artifact",
      project_public_id: "project",
      scope_kind: "project",
      label: "[FICTIF] Spécification",
      status: "active",
      version_public_id: null,
      source_url: "https://www.notion.so/example",
    },
  ],
  edges: [
    {
      id: "edge",
      source_kind: "artifact",
      source_public_id: "spec",
      source_project_public_id: "project",
      target_kind: "knowledge",
      target_public_id: "version-1",
      target_project_public_id: "project",
      edge_type: "derived_from",
      status: "confirmed",
      provenance: { origin: "human" },
    },
  ],
};

afterEach(cleanup);
describe("graph explorer", () => {
  it("distinguishes same-title versions and identifies historical sources", () => {
    const versions: GraphView = {
      ...graph,
      edges: [],
      nodes: [1, 2].map((version) => ({
        ...graph.nodes[1],
        id: `artifact-version-${version}`,
        version_public_id: `artifact-version-${version}`,
        version_number: version,
        status: version === 1 ? "superseded" : "validated",
        source_url: `https://example.invalid/spec?version=${version}`,
      })),
    };
    render(
      <MemoryRouter>
        <GraphExplorer
          graph={versions}
          scopeNames={{ project: "[FICTIF] Boutique" }}
        />
      </MemoryRouter>,
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: "[FICTIF] Spécification, version 1, Livrable, [FICTIF] Boutique",
      }),
    );
    expect(screen.getByText("Version 1 · Historique")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /Ouvrir la source/ })
        .getAttribute("href"),
    ).toBe("https://example.invalid/spec?version=1");
    fireEvent.click(
      screen.getByRole("button", {
        name: "[FICTIF] Spécification, version 2, Livrable, [FICTIF] Boutique",
      }),
    );
    expect(screen.getByText("Version 2 · Validée")).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /Ouvrir la source/ })
        .getAttribute("href"),
    ).toBe("https://example.invalid/spec?version=2");
  });
  it("opens real relationships and source links from either graph or list", () => {
    render(
      <MemoryRouter>
        <GraphExplorer
          graph={graph}
          scopeNames={{ project: "[FICTIF] Boutique" }}
        />
      </MemoryRouter>,
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: "[FICTIF] Spécification, Livrable, [FICTIF] Boutique",
      }),
    );
    expect(
      screen.getByRole("heading", { name: "[FICTIF] Spécification" }),
    ).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /Ouvrir la source/ })
        .getAttribute("href"),
    ).toBe("https://www.notion.so/example");
    fireEvent.click(
      screen.getByRole("button", { name: "[FICTIF] Livraison en France" }),
    );
    expect(
      screen.getByRole("heading", { name: "[FICTIF] Livraison en France" }),
    ).toBeTruthy();
    expect(screen.queryByRole("link", { name: /Ouvrir la source/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Liste" }));
    expect(
      screen.getByRole("list", { name: "Éléments du graphe" }),
    ).toBeTruthy();
    expect(screen.getByText("Version de la source")).toBeTruthy();
  });
  it("clears hidden selection on filtering and describes limited scope honestly", () => {
    render(
      <MemoryRouter>
        <GraphExplorer graph={{ ...graph, truncated: true }} scopeNames={{}} />
      </MemoryRouter>,
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: "[FICTIF] Livraison en France, Connaissance, Projet",
      }),
    );
    fireEvent.change(
      screen.getByRole("textbox", { name: "Rechercher dans le graphe" }),
      { target: { value: "introuvable" } },
    );
    expect(
      screen.queryByRole("heading", { name: "[FICTIF] Livraison en France" }),
    ).toBeNull();
    expect(screen.getByText("Aucun élément ne correspond")).toBeTruthy();
    expect(
      screen.getByText(/Cette vue est limitée à 200 éléments/),
    ).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Effacer les filtres" }),
    );
    expect(
      screen.getByRole("button", {
        name: "[FICTIF] Livraison en France, Connaissance, Projet",
      }),
    ).toBeTruthy();
  });
});
