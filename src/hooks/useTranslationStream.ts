import { useState, useCallback, useEffect, useRef } from "react";
import { Channel } from "@tauri-apps/api/core";
import { invokeWithTimeout } from "@/utils/invokeWithTimeout";
import { normalizeSourceLanguage } from "@/utils/languageArgs";
import type { TranslateStreamEvent, Language, TranslateResponse } from "@/types";

const NOOP_TRANSLATE_HANDLER = () => {};

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
  // Refs for lifecycle safety and stale-event guards
  const activeChannelRef = useRef<Channel<TranslateStreamEvent> | null>(null);
  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  const detachActiveChannel = useCallback(() => {
    if (!activeChannelRef.current) {
      return;
    }
    activeChannelRef.current.onmessage = NOOP_TRANSLATE_HANDLER;
    activeChannelRef.current = null;
  }, []);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
      requestIdRef.current += 1;
      isTranslatingRef.current = false;
      detachActiveChannel();
    };
  }, [detachActiveChannel]);

  const translate = useCallback(
    async (text: string, model?: string): Promise<void> => {
      // Prevent concurrent calls
      if (isTranslatingRef.current) {
        return;
      }
      isTranslatingRef.current = true;
      const requestId = ++requestIdRef.current;
      const isStaleRequest = () => !isMountedRef.current || requestId !== requestIdRef.current;
      detachActiveChannel();

      setError(null);

      try {
        // Fast path: if cached, bypass streaming channel entirely.
        const cached = await invokeWithTimeout<TranslateResponse | null>("get_cached_translation", {
          text,
          sourceLanguage: normalizeSourceLanguage(options.sourceLanguage),
          targetLanguage: options.targetLanguage,
          model,
        });

        if (isStaleRequest()) {
          return;
        }

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
          detachActiveChannel();
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
        activeChannelRef.current = channel;

        channel.onmessage = (event: TranslateStreamEvent) => {
          if (isStaleRequest()) {
            return;
          }

          switch (event.event) {
            case "started":
              setDetectedLanguage(event.data.detectedLanguage);
              setFromCache(event.data.fromCache);
              setGlossaryApplied(event.data.glossaryApplied);
              break;
            case "delta":
              accumulatedTextRef.current += event.data.text;
              setStreamedText(accumulatedTextRef.current);
              break;
            case "completed":
            {
              // Use accumulated text as fallback if fullText is undefined
              const finalText = event.data.fullText ?? accumulatedTextRef.current;
              setFullText(finalText);
              setStreamedText(finalText);
              setTokenUsage(event.data.tokenUsage);
              setIsStreaming(false);
              if (activeChannelRef.current === channel) {
                detachActiveChannel();
              }
              break;
            }
            case "error":
              setError(event.data.message);
              setStreamedText("");
              setFullText("");
              setIsStreaming(false);
              if (activeChannelRef.current === channel) {
                detachActiveChannel();
              }
              break;
          }
        };

        await invokeWithTimeout("translate_stream", {
          text,
          sourceLanguage: normalizeSourceLanguage(options.sourceLanguage),
          targetLanguage: options.targetLanguage,
          model,
          onEvent: channel,
        });

        if (isStaleRequest()) {
          return;
        }
      } catch (err) {
        if (isStaleRequest()) {
          return;
        }
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setIsStreaming(false);
      } finally {
        if (requestId === requestIdRef.current) {
          isTranslatingRef.current = false;
        }
      }
    },
    [detachActiveChannel, options.sourceLanguage, options.targetLanguage]
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
