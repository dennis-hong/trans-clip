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

  it("shows cached translations delivered through the stream", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "translate_stream") {
        const channel = args.onEvent as {
          onmessage?: (event: { event: string; data: Record<string, unknown> }) => void;
        };
        channel.onmessage?.({
          event: "started",
          data: { detectedLanguage: "en", fromCache: true, glossaryApplied: ["g1"] },
        });
        channel.onmessage?.({
          event: "completed",
          data: { fullText: "안녕하세요", tokenUsage: { inputTokens: 1, outputTokens: 2 } },
        });
      }
      return undefined;
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
    expect(result.current.isStreaming).toBe(false);
    // One IPC round trip: no separate cache lookup before streaming.
    expect(invokeWithTimeoutMock).toHaveBeenCalledTimes(1);
    expect(invokeWithTimeoutMock).toHaveBeenCalledWith(
      "translate_stream",
      expect.objectContaining({ text: "hello" }),
      120_000
    );
  });

  it("accumulates streaming deltas and completes", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
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

  it("lets the latest translate call supersede an in-flight one", async () => {
    type Channel = {
      onmessage?: (event: { event: string; data: Record<string, unknown> }) => void;
    };
    const channels: Channel[] = [];
    const resolvers: Array<() => void> = [];
    invokeWithTimeoutMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === "translate_stream") {
        channels.push(args.onEvent as Channel);
        return new Promise<void>((resolve) => {
          resolvers.push(resolve);
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useTranslationStream());

    let firstPromise: Promise<void> | undefined;
    let secondPromise: Promise<void> | undefined;
    await act(async () => {
      firstPromise = result.current.translate("first");
      await Promise.resolve();
      secondPromise = result.current.translate("second");
      await Promise.resolve();
    });

    // Both requests reached the backend; the second is not dropped.
    expect(invokeWithTimeoutMock).toHaveBeenCalledTimes(2);
    expect(invokeWithTimeoutMock).toHaveBeenLastCalledWith(
      "translate_stream",
      expect.objectContaining({ text: "second" }),
      120_000
    );

    await act(async () => {
      // The superseded stream's late events must be ignored.
      channels[0].onmessage?.({ event: "delta", data: { text: "stale" } });
      channels[1].onmessage?.({ event: "delta", data: { text: "fresh" } });
      channels[1].onmessage?.({
        event: "completed",
        data: { fullText: "fresh", tokenUsage: null },
      });
      resolvers.forEach((resolve) => resolve());
      await Promise.all([firstPromise, secondPromise]);
    });

    expect(result.current.streamedText).toBe("fresh");
    expect(result.current.fullText).toBe("fresh");
    expect(result.current.isStreaming).toBe(false);
    expect(result.current.error).toBeNull();
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
