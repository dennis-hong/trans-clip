import { useState, useCallback, useEffect, useRef } from "react";
import { Channel } from "@tauri-apps/api/core";
import { invokeWithTimeout } from "@/utils/invokeWithTimeout";
import type { PolishStreamEvent, PolishContext, PolishChannel, PolishOption, Language } from "@/types";

const NOOP_POLISH_HANDLER = () => {};

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
  // Refs for lifecycle safety and stale-event guards
  const activeChannelRef = useRef<Channel<PolishStreamEvent> | null>(null);
  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  const detachActiveChannel = useCallback(() => {
    if (!activeChannelRef.current) {
      return;
    }
    activeChannelRef.current.onmessage = NOOP_POLISH_HANDLER;
    activeChannelRef.current = null;
  }, []);

  const invalidateActiveRequest = useCallback(() => {
    requestIdRef.current += 1;
    isPolishingRef.current = false;
    detachActiveChannel();
  }, [detachActiveChannel]);

  useEffect(() => {
    // React StrictMode (dev) runs effect cleanup + setup twice.
    // Reset mount flag on each setup so stale checks remain valid.
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      invalidateActiveRequest();
    };
  }, [invalidateActiveRequest]);

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
      const requestId = ++requestIdRef.current;
      const isStaleRequest = () => !isMountedRef.current || requestId !== requestIdRef.current;
      detachActiveChannel();

      setIsStreaming(true);
      setStreamedText("");
      setFullText("");
      setError(null);
      setTokenUsage(null);
      setDetectedLanguage(null);
      accumulatedTextRef.current = "";

      try {
        const channel = new Channel<PolishStreamEvent>();
        activeChannelRef.current = channel;

        channel.onmessage = (event: PolishStreamEvent) => {
          if (isStaleRequest()) {
            return;
          }

          switch (event.event) {
            case "started":
              setDetectedLanguage(event.data.detectedLanguage);
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

        await invokeWithTimeout("polish_stream", {
          text,
          context,
          channel: polishChannel,
          options,
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
        if (requestId === requestIdRef.current) {
          detachActiveChannel();
        }
      } finally {
        if (requestId === requestIdRef.current) {
          isPolishingRef.current = false;
        }
      }
    },
    [detachActiveChannel]
  );

  const clearResult = useCallback(() => {
    invalidateActiveRequest();
    setIsStreaming(false);
    setStreamedText("");
    setFullText("");
    setError(null);
    setTokenUsage(null);
    setDetectedLanguage(null);
  }, [invalidateActiveRequest]);

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
