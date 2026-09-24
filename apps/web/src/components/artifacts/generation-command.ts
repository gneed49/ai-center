import type { ArtifactType } from "@/api/artifact-types";
import { draftStorageIdentity } from "@/api/request-context";
export interface GenerationCommand {
  key: string;
  createdAt: number;
  input: {
    artifact_type: ArtifactType;
    session_id: string;
    instructions: string;
  };
}
export function generationStorageKey(project: string, type: ArtifactType) {
  const identity = draftStorageIdentity();
  return `ai-center.artifact-generation:${identity.actorId}:${identity.workspaceId}:${project}:${type}`;
}
export function readGenerationCommand(
  key: string,
  type: ArtifactType,
): GenerationCommand | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw || raw.length > 24000) return null;
    const item = JSON.parse(raw) as GenerationCommand;
    if (
      typeof item.key !== "string" ||
      !/^[a-f0-9-]{36}$/i.test(item.key) ||
      !Number.isFinite(item.createdAt) ||
      item.input.artifact_type !== type ||
      typeof item.input.session_id !== "string" ||
      typeof item.input.instructions !== "string" ||
      item.input.instructions.length > 8000
    )
      return null;
    return item;
  } catch {
    return null;
  }
}
export function saveGenerationCommand(key: string, command: GenerationCommand) {
  // Only the user's draft and opaque command identity; no Auth/provider secret.
  localStorage.setItem(key, JSON.stringify(command));
}
