import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TranslateResponse, Language } from "@/types";

interface UseTranslationOptions {
  sourceLanguage?: Language | "auto";
  targetLanguage?: Language;
}

interface UseTranslationReturn {
  translate: (text: string) => Promise<TranslateResponse>;
  isLoading: boolean;
  error: string | null;
  result: TranslateResponse | null;
  clearResult: () => void;
}

export function useTranslation(
  options: UseTranslationOptions = {}
): UseTranslationReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<TranslateResponse | null>(null);

  const translate = useCallback(
    async (text: string): Promise<TranslateResponse> => {
      setIsLoading(true);
      setError(null);

      try {
        const response = await invoke<TranslateResponse>("translate", {
          text,
          sourceLanguage:
            options.sourceLanguage === "auto"
              ? undefined
              : options.sourceLanguage,
          targetLanguage: options.targetLanguage,
        });

        setResult(response);

        if (!response.success && response.error) {
          setError(response.error.message);
        }

        return response;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        const errorResponse: TranslateResponse = {
          success: false,
          fromCache: false,
          glossaryApplied: [],
          error: {
            code: "NETWORK_ERROR",
            message,
          },
        };
        setResult(errorResponse);
        return errorResponse;
      } finally {
        setIsLoading(false);
      }
    },
    [options.sourceLanguage, options.targetLanguage]
  );

  const clearResult = useCallback(() => {
    setResult(null);
    setError(null);
  }, []);

  return {
    translate,
    isLoading,
    error,
    result,
    clearResult,
  };
}
