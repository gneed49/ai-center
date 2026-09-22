import type { ContextPackExportFormat } from "@/api/client";
import type { ContextPackSummary } from "@/api/types";

export function downloadContextPack(
  pack: ContextPackSummary,
  format: ContextPackExportFormat,
  content: string,
) {
  const extension = format === "markdown" ? "md" : "json";
  const blob = new Blob([content], {
    type:
      format === "markdown"
        ? "text/markdown;charset=utf-8"
        : "application/json;charset=utf-8",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `context-pack-${pack.public_id}-v${pack.version}.${extension}`;
  document.body.append(anchor);
  try {
    anchor.click();
  } finally {
    anchor.remove();
    // The browser must start the download before the temporary URL is revoked.
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }
}
