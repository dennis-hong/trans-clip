import { useState, useCallback, useRef } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import type { TranslateStreamEvent, Language, TranslateResponse } from "@/types";

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
  // Refs to detect when channel events are actually received
  const receivedAnyEventRef = useRef(false);
  const receivedFinalEventRef = useRef(false);

  const translate = useCallback(
    async (text: string, model?: string): Promise<void> => {
      // Prevent concurrent calls
      if (isTranslatingRef.current) {
        return;
      }
      isTranslatingRef.current = true;
      receivedAnyEventRef.current = false;
      receivedFinalEventRef.current = false;

      setError(null);

      try {
        // Fast path: if cached, bypass streaming channel entirely.
        const cached = await invoke<TranslateResponse | null>("get_cached_translation", {
          text,
          sourceLanguage:
            options.sourceLanguage === "auto"
              ? undefined
              : options.sourceLanguage,
          targetLanguage: options.targetLanguage,
          model,
        });

        if (cached) {
          setDetectedLanguage(cached.detectedLanguage ?? null);
          setFromCache(cached.fromCache);
          setGlossaryApplied(cached.glossaryApplied ?? []);
          setTokenUsage(cached.tokenUsage ?? null);

          const translatedText = cached.translatedText ?? "";
          setFullText(translatedText);
          setStreamedText(translatedText);

          if (!cached.success && cached.error) {
            setError(cached.error.message);
          } else {
            setError(null);
          }

          setIsStreaming(false);
          return;
        }

        // Streaming path (cache miss)
        setIsStreaming(true);
        setStreamedText("");
        setFullText("");
        setFromCache(false);
        setGlossaryApplied([]);
        setTokenUsage(null);
        setDetectedLanguage(null);
        accumulatedTextRef.current = "";

        const channel = new Channel<TranslateStreamEvent>();

        channel.onmessage = (event: TranslateStreamEvent) => {
          receivedAnyEventRef.current = true;
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
              receivedFinalEventRef.current = true;
              // Use accumulated text as fallback if fullText is undefined
              const finalText = event.data.fullText ?? accumulatedTextRef.current;
              setFullText(finalText);
              setStreamedText(finalText);
              setTokenUsage(event.data.tokenUsage);
              setIsStreaming(false);
              break;
            case "error":
              receivedFinalEventRef.current = true;
              setError(event.data.message);
              setStreamedText("");
              setFullText("");
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
