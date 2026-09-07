// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { providersApi } from "@/api/providers";
import { SubscriptionPanel } from "./subscription-panel";

vi.mock("@/api/providers", () => ({
  providersApi: {
    subscriptionStatus: vi.fn(),
    startLogin: vi.fn(),
    loginStatus: vi.fn(),
    cancelLogin: vi.fn(),
    logout: vi.fn(),
  },
}));
const disconnected = {
  available: true,
  authenticated: false,
  connected: false,
  status: "disconnected" as const,
};
const connected = {
  available: true,
  authenticated: true,
  connected: true,
  status: "connected" as const,
};
const pending = {
  login_id: "fixture-login",
  status: "pending" as const,
  auth_url: "https://claude.com/cai/oauth/authorize?state=fixture-transient",
};

beforeEach(() => {
  vi.resetAllMocks();
  vi.useFakeTimers();
  vi.mocked(providersApi.subscriptionStatus).mockResolvedValue(disconnected);
  vi.mocked(providersApi.startLogin).mockResolvedValue(pending);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
async function mount() {
  let result!: ReturnType<typeof render>;
  await act(async () => {
    result = render(
      <SubscriptionPanel connectionId="fixture-connection" available />,
    );
  });
  return result;
}
async function start() {
  await act(async () => {
    fireEvent.click(
      screen.getByRole("button", { name: "Connecter mon abonnement" }),
    );
  });
}

describe("subscription component with a simulated API", () => {
  it("keeps an official login URL transient, stops polling on completion, and removes the URL", async () => {
    vi.mocked(providersApi.loginStatus).mockResolvedValue({
      ...pending,
      status: "connected",
    });
    await mount();
    await start();
    expect(screen.getByRole<HTMLAnchorElement>("link").href).toBe(
      pending.auth_url,
    );
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByRole("status").textContent).toBe("Abonnement connecté.");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(providersApi.loginStatus).toHaveBeenCalledTimes(1);
  });
  it("cancels a login and removes its link", async () => {
    vi.mocked(providersApi.cancelLogin).mockResolvedValue({
      ...pending,
      status: "cancelled",
    });
    await mount();
    await start();
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Annuler la connexion" }),
      );
    });
    expect(providersApi.cancelLogin).toHaveBeenCalledWith(
      "fixture-connection",
      "fixture-login",
    );
    expect(screen.queryByRole("link")).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(providersApi.loginStatus).not.toHaveBeenCalled();
  });
  it("never renders an unofficial authorization link", async () => {
    vi.mocked(providersApi.startLogin).mockResolvedValue({
      ...pending,
      auth_url: "https://evil.example/authorize",
    });
    await mount();
    await start();
    expect(screen.queryByRole("link")).toBeNull();
  });
  it("aborts the in-flight poll when the connection scope closes", async () => {
    vi.mocked(providersApi.loginStatus).mockImplementation(
      () => new Promise(() => {}),
    );
    const result = await mount();
    await start();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    const signal = vi.mocked(providersApi.loginStatus).mock.calls[0][2];
    result.unmount();
    expect(signal?.aborted).toBe(true);
  });
  it("reports an expired login without retaining its URL", async () => {
    vi.mocked(providersApi.loginStatus).mockResolvedValue({
      ...pending,
      status: "expired",
    });
    await mount();
    await start();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByRole("status").textContent).toContain("expiré");
  });
  it("disconnects through the server and refreshes the displayed state", async () => {
    vi.mocked(providersApi.subscriptionStatus).mockResolvedValue(connected);
    vi.mocked(providersApi.logout).mockResolvedValue(disconnected);
    await mount();
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Déconnecter l’abonnement" }),
      );
    });
    expect(providersApi.logout).toHaveBeenCalledWith("fixture-connection");
    expect(
      screen.queryByRole("button", { name: "Déconnecter l’abonnement" }),
    ).toBeNull();
  });
  it("does not start the official client for an unavailable capability", async () => {
    await act(async () => {
      render(<SubscriptionPanel connectionId="fixture" available={false} />);
    });
    expect(providersApi.subscriptionStatus).not.toHaveBeenCalled();
    expect(
      screen.getByRole<HTMLButtonElement>("button", {
        name: "Connecter mon abonnement",
      }).disabled,
    ).toBe(true);
  });
  it.each(["API", "Team"])(
    "keeps recovery controls for an authenticated %s account after login",
    async (account) => {
      const unsupported = {
        authenticated: true,
        connected: false,
        available: false,
        status: "unavailable" as const,
        message: `Compte ${account} non admissible.`,
      };
      vi.mocked(providersApi.subscriptionStatus)
        .mockResolvedValueOnce(disconnected)
        .mockResolvedValue(unsupported);
      vi.mocked(providersApi.loginStatus).mockResolvedValue({
        ...pending,
        status: "failed",
      });
      vi.mocked(providersApi.logout).mockResolvedValue(disconnected);
      await mount();
      await start();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(screen.getByRole("status").textContent).toBe(unsupported.message);
      const change = screen.getByRole<HTMLButtonElement>("button", {
        name: "Changer de compte",
      });
      expect(change.disabled).toBe(false);
      await act(async () => {
        fireEvent.click(
          screen.getByRole("button", { name: "Déconnecter l’abonnement" }),
        );
      });
      expect(providersApi.logout).toHaveBeenCalledWith("fixture-connection");
      expect(
        screen.queryByRole("button", { name: "Déconnecter l’abonnement" }),
      ).toBeNull();
      vi.mocked(providersApi.subscriptionStatus).mockResolvedValue(connected);
      vi.mocked(providersApi.loginStatus).mockResolvedValue({
        ...pending,
        status: "connected",
      });
      await start();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(screen.getByRole("status").textContent).toBe(
        "Abonnement connecté.",
      );
    },
  );
  it("starts account replacement while an unsupported account remains authenticated", async () => {
    vi.mocked(providersApi.subscriptionStatus).mockResolvedValue({
      authenticated: true,
      connected: false,
      available: false,
      status: "unavailable",
      message: "Compte non admissible.",
    });
    await mount();
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Changer de compte" }),
      );
    });
    expect(providersApi.startLogin).toHaveBeenCalledWith("fixture-connection");
    expect(
      screen.getByRole("button", { name: "Annuler la connexion" }),
    ).toBeDefined();
  });
  it("keeps recovery available when logout cannot verify disconnection", async () => {
    vi.mocked(providersApi.subscriptionStatus).mockResolvedValue({
      authenticated: true,
      connected: false,
      available: false,
      status: "unavailable",
      message: "Compte non admissible.",
    });
    vi.mocked(providersApi.logout).mockRejectedValue(
      new Error("fixture failure"),
    );
    await mount();
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Déconnecter l’abonnement" }),
      );
    });
    expect(
      screen.getByRole<HTMLButtonElement>("button", {
        name: "Déconnecter l’abonnement",
      }).disabled,
    ).toBe(false);
    expect(
      screen.getByRole<HTMLButtonElement>("button", {
        name: "Changer de compte",
      }).disabled,
    ).toBe(false);
    expect(screen.getByRole("status").textContent).toBe(
      "Compte non admissible.",
    );
  });
});
