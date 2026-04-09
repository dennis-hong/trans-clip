import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface HideAndPasteResponse {
  success: boolean;
  error?: {
    code: string;
    message: string;
  };
}

async function restorePopupWindow() {
  const window = getCurrentWindow();

  try {
    await window.show();
  } catch (err) {
    console.error("Failed to restore hidden popup window:", err);
  }

  try {
    await window.setFocus();
  } catch (err) {
    console.error("Failed to focus restored popup window:", err);
  }
}

export async function hideAndPasteText(text: string): Promise<HideAndPasteResponse> {
  try {
    const response = await invoke<HideAndPasteResponse>("hide_and_paste_text", { text });

    if (!response.success) {
      await restorePopupWindow();
    }

    return response;
  } catch (err) {
    await restorePopupWindow();
    throw err;
  }
}
