import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranslationStream } from "./useTranslationStream";

const invokeWithTimeoutMock = vi.fn();

vi.mock("@/utils/invokeWithTimeout", () => ({
  invokeWithTimeout: (...args: unknown[]) => invokeWithTimeoutMock(...args),
}));

vi.mock("@tauri-apps/api/core", () => {
  class MockChannel<T> {
    onmessage?: (event: T) => void;
  }
  return { Channel: MockChannel };
});

describe("useTranslationStream", () => {
  beforeEach(() => {
    invokeWithTimeoutMock.mockReset();
  });

  it("uses cache path when cached translation exists", async () => {
    invokeWithTimeoutMock.mockResolvedValueOnce({
      success: true,
      translatedText: "안녕하세요",
      detectedLanguage: "en",
      fromCache: true,
      glossaryApplied: ["g1"],
      tokenUsage: { inputTokens: 1, outputTokens: 2 },
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    expect(result.current.fromCache).toBe(true);
    expect(result.current.fullText).toBe("안녕하세요");
    expect(result.current.streamedText).toBe("안녕하세요");
    expect(result.current.detectedLanguage).toBe("en");
    expect(result.current.glossaryApplied).toEqual(["g1"]);
    expect(result.current.error).toBeNull();
    expect(invokeWithTimeoutMock).toHaveBeenCalledTimes(1);
  });

  it("accumulates streaming deltas and completes", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "get_cached_translation") {
        return null;
      }
      if (cmd === "translate_stream") {
        const channel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        channel.onmessage?.({
          event: "started",
          data: {
            detectedLanguage: "en",
            fromCache: false,
            glossaryApplied: [],
          },
        });
        channel.onmessage?.({ event: "delta", data: { text: "안" } });
        channel.onmessage?.({ event: "delta", data: { text: "녕" } });
        channel.onmessage?.({
          event: "completed",
          data: {
            fullText: "안녕",
            tokenUsage: { inputTokens: 10, outputTokens: 20 },
          },
        });
      }
      return undefined;
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    expect(result.current.fromCache).toBe(false);
    expect(result.current.streamedText).toBe("안녕");
    expect(result.current.fullText).toBe("안녕");
    expect(result.current.isStreaming).toBe(false);
    expect(result.current.tokenUsage).toEqual({ inputTokens: 10, outputTokens: 20 });
  });

  it("ignores concurrent translate calls", async () => {
    let resolveStream: (() => void) | undefined;
    invokeWithTimeoutMock.mockImplementation((cmd: string) => {
      if (cmd === "get_cached_translation") {
        return Promise.resolve(null);
      }
      if (cmd === "translate_stream") {
        return new Promise<void>((resolve) => {
          resolveStream = resolve;
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useTranslationStream());

    let firstPromise: Promise<void> | undefined;
    await act(async () => {
      firstPromise = result.current.translate("first");
      await Promise.resolve();
      await result.current.translate("second");
    });

    expect(invokeWithTimeoutMock).toHaveBeenCalledTimes(2);

    resolveStream?.();
    await firstPromise;
  });
});
