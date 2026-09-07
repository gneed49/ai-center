import { useEffect, useRef, useState } from "react";
import { providerError } from "@/lib/provider-settings";

export function useProviderAction() {
  const mounted = useRef(false);
  const running = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  async function run<T>(action: () => Promise<T>, done: (result: T) => void) {
    if (running.current) return;
    running.current = true;
    setBusy(true);
    setError(undefined);
    try {
      const result = await action();
      if (mounted.current) done(result);
    } catch (cause) {
      if (mounted.current) setError(providerError(cause));
    } finally {
      running.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  function assertActive() {
    if (!mounted.current)
      throw new DOMException("Formulaire fermé", "AbortError");
  }
  return { busy, error, run, assertActive };
}
