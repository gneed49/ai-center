// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { fixtureCode } from "@/test-fixtures/code-evidence";
import { CodeObservationView } from "./observation-view";
const writeText = vi.fn();
beforeEach(() => {
  writeText.mockReset().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
});
afterEach(cleanup);
describe("exact observed source evidence", () => {
  it("copies commit, path, exact lines and immutable observation provenance", async () => {
    render(<CodeObservationView detail={fixtureCode} />);
    fireEvent.change(screen.getByLabelText("Première ligne"), {
      target: { value: "2" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Copier la référence" }),
    );
    await screen.findByText("Référence exacte copiée.");
    expect(writeText).toHaveBeenCalledWith(
      expect.stringContaining(
        `fiction/example@${fixtureCode.corpus.commit_sha}:src/fixture.ts:L2-L3`,
      ),
    );
    expect(writeText).toHaveBeenCalledWith(
      expect.stringContaining("Observation AI Center fixture-file"),
    );
    expect(
      screen
        .getByRole("link", { name: "Voir ces lignes sur GitHub" })
        .getAttribute("href"),
    ).toContain("#L2-L3");
    fireEvent.change(screen.getByLabelText("Dernière ligne"), {
      target: { value: "99" },
    });
    expect(
      (
        screen.getByRole("button", {
          name: "Copier la référence",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(
      screen.queryByRole("link", { name: "Voir ces lignes sur GitHub" }),
    ).toBeNull();
  });
  it("keeps unavailable source uncertainty explicit, without code or citation controls", () => {
    render(
      <CodeObservationView
        detail={{
          ...fixtureCode,
          corpus: { ...fixtureCode.corpus, commit_verified: false },
          files: [
            {
              ...fixtureCode.files[0],
              status: "unavailable",
              content_text: null,
              line_count: 0,
              reason_code: "remote_timeout",
            },
          ],
        }}
      />,
    );
    expect(screen.getByText(/Commit non vérifié/)).toBeDefined();
    expect(
      screen.getByText(/Le contenu de ce fichier n’a pas été lu/),
    ).toBeDefined();
    expect(
      screen.queryByRole("button", { name: "Copier la référence" }),
    ).toBeNull();
    expect(
      screen.getByText(/ne vérifie pas le fonctionnement du dépôt entier/),
    ).toBeDefined();
  });
});

it("explains the exclusion of possible credentials without showing any source content", () => {
  render(
    <CodeObservationView
      detail={{
        ...fixtureCode,
        files: [
          {
            ...fixtureCode.files[0],
            status: "unsupported",
            reason_code: "sensitive_content_excluded",
            content_text: null,
            blob_sha: null,
            content_hash: null,
            line_count: 0,
          },
        ],
      }}
    />,
  );
  expect(screen.getAllByText("Contenu sensible exclu").length).toBeGreaterThan(
    0,
  );
  expect(
    screen.getByText(
      /peut contenir une clé ou un identifiant d’accès sensible/,
    ),
  ).toBeDefined();
  expect(
    screen.queryByRole("button", { name: "Copier la référence" }),
  ).toBeNull();
});
