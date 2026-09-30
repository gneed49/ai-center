// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { companyApi } from "@/api/company";
import { CompanyBootstrap } from "./company-bootstrap";
vi.mock("@/api/company", () => ({
  companyApi: { capabilities: vi.fn(), create: vi.fn() },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
});
afterEach(() => {
  cleanup();
  client.clear();
});
function page() {
  render(
    <QueryClientProvider client={client}>
      <CompanyBootstrap onCreated={vi.fn()} />
    </QueryClientProvider>,
  );
}
describe("private company bootstrap", () => {
  it("guides an unapproved identity to an invitation without a creation form", async () => {
    vi.mocked(companyApi.capabilities).mockResolvedValue({
      can_create_company: false,
    });
    page();
    await screen.findByRole("heading", { name: "Rejoindre votre entreprise" });
    expect(
      screen.queryByRole("textbox", { name: "Nom de l’entreprise" }),
    ).toBeNull();
    expect(companyApi.create).not.toHaveBeenCalled();
  });
  it("offers creation when the server grants the capability", async () => {
    vi.mocked(companyApi.capabilities).mockResolvedValue({
      can_create_company: true,
    });
    page();
    await screen.findByRole("textbox", { name: "Nom de l’entreprise" });
  });
});
