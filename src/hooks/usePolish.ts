import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PolishResponse, PolishContext, PolishChannel, PolishOption } from "@/types";

interface UsePolishReturn {
  polish: (
    text: string,
    context: PolishContext,
    channel: PolishChannel,
    options: PolishOption[],
    model?: string
  ) => Promise<PolishResponse>;
  isLoading: boolean;
  error: string | null;
  result: PolishResponse | null;
  clearResult: () => void;
}

export function usePolish(): UsePolishReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PolishResponse | null>(null);

  const polish = useCallback(
    async (
      text: string,
      context: PolishContext,
      channel: PolishChannel,
      options: PolishOption[],
      model?: string
    ): Promise<PolishResponse> => {
      setIsLoading(true);
      setError(null);

      try {
        const response = await invoke<PolishResponse>("polish", {
          text,
          context,
          channel,
          options,
          model,
        });

        setResult(response);

        if (!response.success && response.error) {
          setError(response.error.message);
        }

        return response;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        const errorResponse: PolishResponse = {
          success: false,
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
    []
  );

  const clearResult = useCallback(() => {
    setResult(null);
    setError(null);
  }, []);

  return {
    polish,
    isLoading,
    error,
    result,
    clearResult,
  };
}
