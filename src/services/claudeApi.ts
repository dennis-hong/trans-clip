import { invoke } from "@tauri-apps/api/core";
import { normalizeSourceLanguage } from "@/utils/languageArgs";
import type {
  TranslateResponse,
  Language,
  ApiKeyStatus,
  SetApiKeyResponse,
} from "@/types";

/**
 * Claude API service wrapper for translation operations
 */
export const claudeApi = {
  /**
   * Translate text using Claude API
   */
  async translate(
    text: string,
    options?: {
      sourceLanguage?: Language | "auto";
      targetLanguage?: Language;
    }
  ): Promise<TranslateResponse> {
    return invoke<TranslateResponse>("translate", {
      text,
      sourceLanguage: normalizeSourceLanguage(options?.sourceLanguage),
      targetLanguage: options?.targetLanguage,
    });
  },

  /**
   * Check API key status
   */
  async getApiKeyStatus(): Promise<ApiKeyStatus> {
    return invoke<ApiKeyStatus>("get_api_key");
  },

  /**
   * Set API key
   */
  async setApiKey(apiKey: string): Promise<SetApiKeyResponse> {
    return invoke<SetApiKeyResponse>("set_api_key", { apiKey });
  },

  /**
   * Delete API key
   */
  async deleteApiKey(): Promise<{ success: boolean }> {
    return invoke<{ success: boolean }>("delete_api_key");
  },

  /**
   * Validate API key format (client-side)
   */
  validateApiKeyFormat(apiKey: string): boolean {
    return apiKey.startsWith("sk-ant-") && apiKey.length > 20;
  },
};
