import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePolishStream } from "./usePolishStream";

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
});
