import { useEffect, useRef, type ReactNode } from "react";

/** Focus once when an explicitly requested source page becomes readable. */
export function SourcePageFrame({ children }: { children: ReactNode }) {
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const title = container.current?.querySelector("h1");
    if (title) {
      title.tabIndex = -1;
      title.focus({ preventScroll: true });
    }
  }, []);
  return (
    <div ref={container} className="min-w-0 space-y-6 [overflow-wrap:anywhere]">
      {children}
    </div>
  );
}
