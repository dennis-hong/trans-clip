import { useState, useCallback, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import type { TranslateStreamEvent, Language } from "@/types";

interface UseTranslationStreamOptions {
  sourceLanguage?: Language | "auto";
  targetLanguage?: Language;
}

interface UseTranslationStreamReturn {
  translate: (text: string, model?: string) => Promise<void>;
  isStreaming: boolean;
  streamedText: string;
  fullText: string;
  detectedLanguage: Language | null;
  fromCache: boolean;
  glossaryApplied: string[];
  tokenUsage: { inputTokens: number; outputTokens: number } | null;
  error: string | null;
  clearResult: () => void;
}

export function useTranslationStream(
  options: UseTranslationStreamOptions = {}
): UseTranslationStreamReturn {
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamedText, setStreamedText] = useState("");
  const [fullText, setFullText] = useState("");
  const [detectedLanguage, setDetectedLanguage] = useState<Language | null>(null);
  const [fromCache, setFromCache] = useState(false);
  const [glossaryApplied, setGlossaryApplied] = useState<string[]>([]);
  const [tokenUsage, setTokenUsage] = useState<{ inputTokens: number; outputTokens: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Ref to accumulate streamed text during a single translate call
  const accumulatedTextRef = useRef("");
  // Ref to prevent concurrent translate calls
  const isTranslatingRef = useRef(false);

  const translate = useCallback(
    async (text: string, model?: string): Promise<void> => {
      // Prevent concurrent calls
      if (isTranslatingRef.current) {
        return;
      }
      isTranslatingRef.current = true;

      setIsStreaming(true);
      setStreamedText("");
      setFullText("");
      setError(null);
      setFromCache(false);
      setGlossaryApplied([]);
      setTokenUsage(null);
      setDetectedLanguage(null);
      accumulatedTextRef.current = "";

      try {
        const channel = new Channel<TranslateStreamEvent>();

        channel.onmessage = (event: TranslateStreamEvent) => {
          switch (event.event) {
            case "started":
              setDetectedLanguage(event.data.detectedLanguage as Language | null);
              setFromCache(event.data.fromCache);
              setGlossaryApplied(event.data.glossaryApplied);
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

        await invoke("translate_stream", {
          text,
          sourceLanguage:
            options.sourceLanguage === "auto"
              ? undefined
              : options.sourceLanguage,
          targetLanguage: options.targetLanguage,
          model,
          onEvent: channel,
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setIsStreaming(false);
      } finally {
        isTranslatingRef.current = false;
      }
    },
    [options.sourceLanguage, options.targetLanguage]
  );

  const clearResult = useCallback(() => {
    setStreamedText("");
    setFullText("");
    setError(null);
    setFromCache(false);
    setGlossaryApplied([]);
    setTokenUsage(null);
    setDetectedLanguage(null);
  }, []);

  return {
    translate,
    isStreaming,
    streamedText,
    fullText,
    detectedLanguage,
    fromCache,
    glossaryApplied,
    tokenUsage,
    error,
    clearResult,
  };
}
