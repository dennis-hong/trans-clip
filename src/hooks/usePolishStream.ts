import { useState, useCallback, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import type { PolishStreamEvent, PolishContext, PolishChannel, PolishOption, Language } from "@/types";

interface UsePolishStreamReturn {
  polish: (
    text: string,
    context: PolishContext,
    channel: PolishChannel,
    options: PolishOption[],
    model?: string
  ) => Promise<void>;
  isStreaming: boolean;
  streamedText: string;
  fullText: string;
  detectedLanguage: Language | null;
  tokenUsage: { inputTokens: number; outputTokens: number } | null;
  error: string | null;
  clearResult: () => void;
}

export function usePolishStream(): UsePolishStreamReturn {
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamedText, setStreamedText] = useState("");
  const [fullText, setFullText] = useState("");
  const [detectedLanguage, setDetectedLanguage] = useState<Language | null>(null);
  const [tokenUsage, setTokenUsage] = useState<{ inputTokens: number; outputTokens: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Ref to accumulate streamed text during a single polish call
  const accumulatedTextRef = useRef("");
  // Ref to prevent concurrent polish calls
  const isPolishingRef = useRef(false);

  const polish = useCallback(
    async (
      text: string,
      context: PolishContext,
      polishChannel: PolishChannel,
      options: PolishOption[],
      model?: string
    ): Promise<void> => {
      // Prevent concurrent calls
      if (isPolishingRef.current) {
        return;
      }
      isPolishingRef.current = true;

      setIsStreaming(true);
      setStreamedText("");
      setFullText("");
      setError(null);
      setTokenUsage(null);
      setDetectedLanguage(null);
      accumulatedTextRef.current = "";

      try {
        const channel = new Channel<PolishStreamEvent>();

        channel.onmessage = (event: PolishStreamEvent) => {
          switch (event.event) {
            case "started":
              setDetectedLanguage(event.data.detectedLanguage as Language | null);
              break;
            case "delta":
              accumulatedTextRef.current += event.data.text;
              setStreamedText(accumulatedTextRef.current);
              break;
            case "completed":
              // Use accumulated text as fallback if fullText is undefined
              const finalText = event.data.fullText ?? accumulatedTextRef.current;
              setFullText(finalText);
              setStreamedText(finalText);
              setTokenUsage(event.data.tokenUsage);
              setIsStreaming(false);
              break;
            case "error":
              setError(event.data.message);
              setIsStreaming(false);
              break;
          }
        };

        await invoke("polish_stream", {
          text,
          context,
          channel: polishChannel,
          options,
          model,
          onEvent: channel,
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setIsStreaming(false);
      } finally {
        isPolishingRef.current = false;
      }
    },
    []
  );

  const clearResult = useCallback(() => {
    setStreamedText("");
    setFullText("");
    setError(null);
    setTokenUsage(null);
    setDetectedLanguage(null);
  }, []);

  return {
    polish,
    isStreaming,
    streamedText,
    fullText,
    detectedLanguage,
    tokenUsage,
    error,
    clearResult,
  };
}
