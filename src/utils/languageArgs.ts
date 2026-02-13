import type { Language } from "@/types";

export function normalizeSourceLanguage(
  sourceLanguage?: Language | "auto"
): Language | undefined {
  if (!sourceLanguage || sourceLanguage === "auto") {
    return undefined;
  }
  return sourceLanguage;
}
