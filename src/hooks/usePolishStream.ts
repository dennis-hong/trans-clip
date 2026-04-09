import { useState, useCallback, useEffect, useRef } from "react";
import { Channel } from "@tauri-apps/api/core";
import { invokeWithTimeout, STREAMING_TIMEOUT_MS } from "@/utils/invokeWithTimeout";
import type {
  PolishStreamEvent,
  PolishResponse,
  PolishContext,
  PolishChannel,
  PolishOption,
  Language,
} from "@/types";

const NOOP_POLISH_HANDLER = () => {};
const EMPTY_POLISH_RESULT_MESSAGE = "다듬기 결과가 비어 있습니다. 다시 시도해 주세요.";

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

  const resetPolishState = useCallback(() => {
    accumulatedTextRef.current = "";
    setIsStreaming(false);
    setStreamedText("");
    setFullText("");
    setError(null);
    setTokenUsage(null);
    setDetectedLanguage(null);
  }, []);

  const applyPolishResponse = useCallback((response: PolishResponse) => {
    setDetectedLanguage(response.detectedLanguage ?? null);
    setTokenUsage(response.tokenUsage ?? null);

    const polishedText = response.polishedText ?? "";
    setFullText(polishedText);
    setStreamedText(polishedText);

    if (!response.success && response.error) {
      setError(response.error.message);
    } else if (!polishedText.trim()) {
      setError(EMPTY_POLISH_RESULT_MESSAGE);
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

  const polish = useCallback(
    async (
      text: string,
      context: PolishContext,
      polishChannel: PolishChannel,
      options: PolishOption[],
      model?: string
    ): Promise<void> => {
      // The latest request should win; otherwise the popup can show a new
      // source text alongside an older polish result.
      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;
      isPolishingRef.current = true;
      const isStaleRequest = () => !isMountedRef.current || requestId !== requestIdRef.current;
      detachActiveChannel();
      resetPolishState();

      setIsStreaming(true);

      try {
        const channel = new Channel<PolishStreamEvent>();
        activeChannelRef.current = channel;
        let terminalEventReceived = false;
        let completedEventReceived = false;
        let completedText = "";

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
              terminalEventReceived = true;
              completedEventReceived = true;
              // Use accumulated text as fallback if fullText is undefined
              const finalText = event.data.fullText ?? accumulatedTextRef.current;
              completedText = finalText;
              setFullText(finalText);
              setStreamedText(finalText);
              setTokenUsage(event.data.tokenUsage);
              if (!finalText.trim()) {
                setError(EMPTY_POLISH_RESULT_MESSAGE);
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

        await invokeWithTimeout("polish_stream", {
          text,
          context,
          channel: polishChannel,
          options,
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

          const fallbackResponse = await invokeWithTimeout<PolishResponse>("polish", {
            text,
            context,
            channel: polishChannel,
            options,
            model,
          }, STREAMING_TIMEOUT_MS);

          if (isStaleRequest()) {
            return;
          }

          applyPolishResponse(fallbackResponse);
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
    [applyPolishResponse, detachActiveChannel, resetPolishState]
  );

  const clearResult = useCallback(() => {
    invalidateActiveRequest();
    resetPolishState();
  }, [invalidateActiveRequest, resetPolishState]);

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
