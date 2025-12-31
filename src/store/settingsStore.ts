import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  UserSettings,
  ApiKeyStatus,
  SetApiKeyResponse,
  DeleteResponse,
} from "@/types";

interface SettingsStore {
  settings: UserSettings | null;
  apiKeyStatus: ApiKeyStatus | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  fetchSettings: () => Promise<void>;
  updateSettings: (settings: Partial<UserSettings>) => Promise<boolean>;
  fetchApiKeyStatus: () => Promise<void>;
  setApiKey: (apiKey: string) => Promise<SetApiKeyResponse>;
  deleteApiKey: () => Promise<boolean>;
  clearError: () => void;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: null,
  apiKeyStatus: null,
  isLoading: false,
  error: null,

  fetchSettings: async () => {
    set({ isLoading: true, error: null });
    try {
      const settings = await invoke<UserSettings>("get_settings");
      set({ settings, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
    }
  },

  updateSettings: async (newSettings: Partial<UserSettings>) => {
    set({ isLoading: true, error: null });
    try {
      const settings = await invoke<UserSettings>("update_settings", {
        settings: newSettings,
      });
      set({ settings, isLoading: false });
      return true;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
      return false;
    }
  },

  fetchApiKeyStatus: async () => {
    try {
      const status = await invoke<ApiKeyStatus>("get_api_key");
      set({ apiKeyStatus: status });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },

  setApiKey: async (apiKey: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<SetApiKeyResponse>("set_api_key", {
        apiKey,
      });

      if (response.success) {
        // Don't immediately re-fetch from keychain here; depending on macOS keychain state
        // this can transiently fail and make the UI look like the key wasn't saved.
        set({
          apiKeyStatus: { exists: true, isValid: response.isValid },
          isLoading: false,
        });
      } else {
        set({
          error: response.error?.message ?? "Failed to set API key",
          isLoading: false,
        });
      }

      return response;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({
        error: message,
        isLoading: false,
      });
      return {
        success: false,
        isValid: false,
        error: { code: "KEYCHAIN_ERROR" as const, message },
      };
    }
  },

  deleteApiKey: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<DeleteResponse>("delete_api_key");

      if (response.success) {
        set({
          apiKeyStatus: { exists: false },
          isLoading: false,
        });
        return true;
      } else {
        set({
          error: response.error?.message ?? "Failed to delete API key",
          isLoading: false,
        });
        return false;
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
      return false;
    }
  },

  clearError: () => set({ error: null }),
}));
