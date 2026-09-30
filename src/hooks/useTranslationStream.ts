import { useState, useCallback, useEffect, useRef } from "react";
import { Channel } from "@tauri-apps/api/core";
import { invokeWithTimeout, STREAMING_TIMEOUT_MS } from "@/utils/invokeWithTimeout";
import { normalizeSourceLanguage } from "@/utils/languageArgs";
import type { TranslateStreamEvent, Language, TranslateResponse } from "@/types";

const NOOP_TRANSLATE_HANDLER = () => {};
const EMPTY_TRANSLATION_RESULT_MESSAGE = "번역 결과가 비어 있습니다. 다시 시도해 주세요.";

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

  const invalidateActiveRequest = useCallback(() => {
    requestIdRef.current += 1;
    detachActiveChannel();
  }, [detachActiveChannel]);

  const applyTranslateResponse = useCallback((response: TranslateResponse) => {
    setDetectedLanguage(response.detectedLanguage ?? null);
    setFromCache(response.fromCache);
    setGlossaryApplied(response.glossaryApplied ?? []);
    setTokenUsage(response.tokenUsage ?? null);

    const translatedText = response.translatedText ?? "";
    setFullText(translatedText);
    setStreamedText(translatedText);

    if (!response.success && response.error) {
      setError(response.error.message);
    } else if (!translatedText.trim()) {
      setError(EMPTY_TRANSLATION_RESULT_MESSAGE);
    } else {
      setError(null);
    }

    setIsStreaming(false);
  }, []);

  useEffect(() => {
    // React StrictMode (dev) runs effect cleanup + setup twice.
    // Reset mount flag on each setup so stale checks remain valid.
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      invalidateActiveRequest();
    };
  }, [invalidateActiveRequest]);

  const translate = useCallback(
    async (text: string, model?: string): Promise<void> => {
      // Latest request wins: a newer call (e.g. new source text arriving while the
      // previous translation is still streaming) supersedes the in-flight one.
      const requestId = ++requestIdRef.current;
      const isStaleRequest = () => !isMountedRef.current || requestId !== requestIdRef.current;
      detachActiveChannel();
      const sourceLanguage = normalizeSourceLanguage(options.sourceLanguage);

      setError(null);
      setIsStreaming(true);
      setStreamedText("");
      setFullText("");
      setFromCache(false);
      setGlossaryApplied([]);
      setTokenUsage(null);
      setDetectedLanguage(null);
      accumulatedTextRef.current = "";

      try {
        // The backend answers cache hits through the same stream (started -> completed),
        // so no separate cache round trip is needed before streaming.
        const channel = new Channel<TranslateStreamEvent>();
        activeChannelRef.current = channel;
        let terminalEventReceived = false;
        let completedEventReceived = false;
        let completedText = "";

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
              terminalEventReceived = true;
              completedEventReceived = true;
              // Use accumulated text as fallback if fullText is undefined
              const finalText = event.data.fullText ?? accumulatedTextRef.current;
              completedText = finalText;
              setFullText(finalText);
              setStreamedText(finalText);
              setTokenUsage(event.data.tokenUsage);
              if (!finalText.trim()) {
                setError(EMPTY_TRANSLATION_RESULT_MESSAGE);
              } else {
                setError(null);
              }
              setIsStreaming(false);
              if (activeChannelRef.current === channel) {
                detachActiveChannel();
              }
              break;
            }
            case "error":
              terminalEventReceived = true;
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
          sourceLanguage,
          targetLanguage: options.targetLanguage,
          model,
          onEvent: channel,
        }, STREAMING_TIMEOUT_MS);

        if (isStaleRequest()) {
          return;
        }

        // Recovery path:
        // If stream invocation returns without terminal events, or completion payload is empty,
        // fall back to the non-streaming command to avoid indefinite loading UI.
        if (!terminalEventReceived || (completedEventReceived && !completedText.trim())) {
          if (activeChannelRef.current === channel) {
            detachActiveChannel();
          }
          setIsStreaming(true);

          const fallbackResponse = await invokeWithTimeout<TranslateResponse>("translate", {
            text,
            sourceLanguage,
            targetLanguage: options.targetLanguage,
            model,
          }, STREAMING_TIMEOUT_MS);

          if (isStaleRequest()) {
            return;
          }

          applyTranslateResponse(fallbackResponse);
        }
      } catch (err) {
        if (isStaleRequest()) {
          return;
        }
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setIsStreaming(false);
        if (requestId === requestIdRef.current) {
          detachActiveChannel();
        }
      }
    },
    [applyTranslateResponse, detachActiveChannel, options.sourceLanguage, options.targetLanguage]
  );

  const clearResult = useCallback(() => {
    invalidateActiveRequest();
    setIsStreaming(false);
    setStreamedText("");
    setFullText("");
    setError(null);
    setFromCache(false);
    setGlossaryApplied([]);
    setTokenUsage(null);
    setDetectedLanguage(null);
  }, [invalidateActiveRequest]);

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
