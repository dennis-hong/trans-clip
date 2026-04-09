import { act, renderHook } from "@testing-library/react";
import { StrictMode, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranslationStream } from "./useTranslationStream";

const invokeWithTimeoutMock = vi.fn();

vi.mock("@/utils/invokeWithTimeout", () => ({
  invokeWithTimeout: (...args: unknown[]) => invokeWithTimeoutMock(...args),
  STREAMING_TIMEOUT_MS: 120_000,
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

  it("falls back to non-streaming translate when stream returns without terminal events", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_cached_translation") {
        return null;
      }
      if (cmd === "translate_stream") {
        return undefined;
      }
      if (cmd === "translate") {
        return {
          success: true,
          translatedText: "안녕하세요",
          detectedLanguage: "en",
          fromCache: false,
          glossaryApplied: [],
          tokenUsage: { inputTokens: 11, outputTokens: 22 },
        };
      }
      return undefined;
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    expect(result.current.fullText).toBe("안녕하세요");
    expect(result.current.streamedText).toBe("안녕하세요");
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
    expect(invokeWithTimeoutMock).toHaveBeenLastCalledWith(
      "translate",
      expect.objectContaining({ text: "hello" }),
      120_000
    );
  });

  it("falls back to non-streaming translate when stream completes with empty text", async () => {
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
        channel.onmessage?.({
          event: "completed",
          data: {
            fullText: "",
            tokenUsage: null,
          },
        });
        return undefined;
      }
      if (cmd === "translate") {
        return {
          success: true,
          translatedText: "fallback result",
          detectedLanguage: "en",
          fromCache: false,
          glossaryApplied: [],
          tokenUsage: null,
        };
      }
      return undefined;
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    expect(result.current.fullText).toBe("fallback result");
    expect(result.current.streamedText).toBe("fallback result");
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
  });

  it("does not stale-out under React StrictMode", async () => {
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
        channel.onmessage?.({
          event: "completed",
          data: {
            fullText: "strict mode ok",
            tokenUsage: null,
          },
        });
      }
      return undefined;
    });

    const wrapper = ({ children }: { children: ReactNode }) => (
      <StrictMode>{children}</StrictMode>
    );
    const { result } = renderHook(() => useTranslationStream(), { wrapper });

    await act(async () => {
      await result.current.translate("hello");
    });

    expect(result.current.fullText).toBe("strict mode ok");
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
  });

  it("supersedes an in-flight translate call with the latest request", async () => {
    let firstChannel:
      | {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        }
      | null = null;
    let resolveStream: (() => void) | undefined;
    invokeWithTimeoutMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_cached_translation") {
        if (args?.text === "first") {
          return Promise.resolve(null);
        }

        if (args?.text === "second") {
          return Promise.resolve({
            success: true,
            translatedText: "두번째 결과",
            detectedLanguage: "en",
            fromCache: true,
            glossaryApplied: [],
            tokenUsage: null,
          });
        }
      }
      if (cmd === "translate_stream" && args?.text === "first") {
        firstChannel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
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

    expect(result.current.fullText).toBe("두번째 결과");
    expect(result.current.streamedText).toBe("두번째 결과");
    expect(result.current.fromCache).toBe(true);

    act(() => {
      firstChannel?.onmessage?.({ event: "delta", data: { text: "stale" } });
    });

    expect(result.current.fullText).toBe("두번째 결과");
    expect(result.current.streamedText).toBe("두번째 결과");

    resolveStream?.();
    await firstPromise;
  });

  it("clears active stream handlers when clearResult is called", async () => {
    let activeChannel:
      | {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        }
      | null = null;

    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "get_cached_translation") {
        return null;
      }
      if (cmd === "translate_stream") {
        activeChannel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
      }
      return undefined;
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    act(() => {
      result.current.clearResult();
    });

    act(() => {
      activeChannel?.onmessage?.({ event: "delta", data: { text: "late" } });
    });

    expect(result.current.streamedText).toBe("");
    expect(result.current.fullText).toBe("");
    expect(result.current.isStreaming).toBe(false);
  });

  it("detaches stream handlers when invoke throws", async () => {
    let activeChannel:
      | {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        }
      | null = null;

    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "get_cached_translation") {
        return null;
      }
      if (cmd === "translate_stream") {
        activeChannel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        throw new Error("invoke timeout");
      }
      return undefined;
    });

    const { result } = renderHook(() => useTranslationStream());

    await act(async () => {
      await result.current.translate("hello");
    });

    act(() => {
      activeChannel?.onmessage?.({ event: "delta", data: { text: "late" } });
    });

    expect(result.current.error).toBe("invoke timeout");
    expect(result.current.streamedText).toBe("");
    expect(result.current.isStreaming).toBe(false);
  });
});
