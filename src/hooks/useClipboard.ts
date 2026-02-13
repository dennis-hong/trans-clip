import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore } from "@/store";
import type { PasteResponse } from "@/types";

export function useClipboard() {
  const store = useClipboardStore();

  const copyToClipboard = useCallback(async (text: string) => {
    try {
      await invoke("set_clipboard", { text });
      return true;
    } catch (err) {
      console.error("Failed to copy:", err);
      return false;
    }
  }, []);

  const pasteText = useCallback(async (text: string) => {
    try {
      const response = await invoke<PasteResponse>("paste_text", { text });
      return response.success;
    } catch (err) {
      console.error("Failed to paste:", err);
      return false;
    }
  }, []);

  return {
    ...store,
    copyToClipboard,
    pasteText,
  };
}
