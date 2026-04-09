import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePolishStream } from "./usePolishStream";

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

describe("usePolishStream", () => {
  beforeEach(() => {
    invokeWithTimeoutMock.mockReset();
  });

  it("streams polish result and finalizes state", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "polish_stream") {
        const channel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        channel.onmessage?.({
          event: "started",
          data: { detectedLanguage: "ko" },
        });
        channel.onmessage?.({ event: "delta", data: { text: "정" } });
        channel.onmessage?.({ event: "delta", data: { text: "리" } });
        channel.onmessage?.({
          event: "completed",
          data: {
            fullText: "정리",
            tokenUsage: { inputTokens: 3, outputTokens: 4 },
          },
        });
      }
      return undefined;
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
    });

    expect(result.current.detectedLanguage).toBe("ko");
    expect(result.current.streamedText).toBe("정리");
    expect(result.current.fullText).toBe("정리");
    expect(result.current.tokenUsage).toEqual({ inputTokens: 3, outputTokens: 4 });
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
  });

  it("sets error on stream error event", async () => {
    invokeWithTimeoutMock.mockImplementation(async (_cmd: string, args: Record<string, unknown>) => {
      const channel = args.onEvent as {
        onmessage?: (event: {
          event: string;
          data: Record<string, unknown>;
        }) => void;
      };
      channel.onmessage?.({
        event: "error",
        data: { code: "API_ERROR", message: "boom" },
      });
      return undefined;
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
    });

    expect(result.current.error).toBe("boom");
    expect(result.current.fullText).toBe("");
    expect(result.current.streamedText).toBe("");
    expect(result.current.isStreaming).toBe(false);
  });

  it("falls back to non-streaming polish when stream returns without terminal events", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string) => {
      if (cmd === "polish_stream") {
        return undefined;
      }
      if (cmd === "polish") {
        return {
          success: true,
          polishedText: "fallback polish result",
          detectedLanguage: "ko",
          tokenUsage: { inputTokens: 7, outputTokens: 8 },
        };
      }
      return undefined;
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
    });

    expect(result.current.fullText).toBe("fallback polish result");
    expect(result.current.streamedText).toBe("fallback polish result");
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
    expect(invokeWithTimeoutMock).toHaveBeenLastCalledWith(
      "polish",
      expect.objectContaining({ text: "draft" }),
      120_000
    );
  });

  it("falls back to non-streaming polish when stream completes with empty text", async () => {
    invokeWithTimeoutMock.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "polish_stream") {
        const channel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        channel.onmessage?.({
          event: "started",
          data: { detectedLanguage: "ko" },
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
      if (cmd === "polish") {
        return {
          success: true,
          polishedText: "fallback from empty completion",
          detectedLanguage: "ko",
          tokenUsage: null,
        };
      }
      return undefined;
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
    });

    expect(result.current.fullText).toBe("fallback from empty completion");
    expect(result.current.streamedText).toBe("fallback from empty completion");
    expect(result.current.error).toBeNull();
    expect(result.current.isStreaming).toBe(false);
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

    invokeWithTimeoutMock.mockImplementation(async (_cmd: string, args: Record<string, unknown>) => {
      activeChannel = args.onEvent as {
        onmessage?: (event: {
          event: string;
          data: Record<string, unknown>;
        }) => void;
      };
      return undefined;
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
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

    invokeWithTimeoutMock.mockImplementation(async (_cmd: string, args: Record<string, unknown>) => {
      activeChannel = args.onEvent as {
        onmessage?: (event: {
          event: string;
          data: Record<string, unknown>;
        }) => void;
      };
      throw new Error("invoke timeout");
    });

    const { result } = renderHook(() => usePolishStream());

    await act(async () => {
      await result.current.polish("draft", "peer-discussion", "slack-message", []);
    });

    act(() => {
      activeChannel?.onmessage?.({ event: "delta", data: { text: "late" } });
    });

    expect(result.current.error).toBe("invoke timeout");
    expect(result.current.streamedText).toBe("");
    expect(result.current.isStreaming).toBe(false);
  });

  it("supersedes an in-flight polish request with the latest one", async () => {
    let firstChannel:
      | {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        }
      | null = null;
    let resolveFirstStream: (() => void) | undefined;

    invokeWithTimeoutMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === "polish_stream" && args.text === "first draft") {
        firstChannel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        return new Promise<void>((resolve) => {
          resolveFirstStream = resolve;
        });
      }

      if (cmd === "polish_stream" && args.text === "second draft") {
        const channel = args.onEvent as {
          onmessage?: (event: {
            event: string;
            data: Record<string, unknown>;
          }) => void;
        };
        channel.onmessage?.({
          event: "started",
          data: { detectedLanguage: "en" },
        });
        channel.onmessage?.({
          event: "completed",
          data: {
            fullText: "latest polish",
            tokenUsage: null,
          },
        });
        return Promise.resolve(undefined);
      }

      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => usePolishStream());

    let firstPromise: Promise<void> | undefined;
    await act(async () => {
      firstPromise = result.current.polish("first draft", "peer-discussion", "slack-message", []);
      await Promise.resolve();
      await result.current.polish("second draft", "peer-discussion", "slack-message", []);
    });

    expect(result.current.fullText).toBe("latest polish");
    expect(result.current.streamedText).toBe("latest polish");
    expect(result.current.detectedLanguage).toBe("en");

    act(() => {
      firstChannel?.onmessage?.({ event: "delta", data: { text: "stale" } });
    });

    expect(result.current.fullText).toBe("latest polish");
    expect(result.current.streamedText).toBe("latest polish");

    resolveFirstStream?.();
    await firstPromise;
  });
});
