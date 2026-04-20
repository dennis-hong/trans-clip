import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HistoryPanel } from "./HistoryPanel";
import { useClipboardStore } from "@/store";

type TauriListenEvent = {
  payload: Record<string, unknown>;
};

type ListenerCallback = (event: TauriListenEvent) => void;

const listeners = new Map<string, ListenerCallback>();
const invokeMock = vi.fn();
const listenMock = vi.fn((eventName: string, callback: ListenerCallback) => {
  listeners.set(eventName, callback);
  return Promise.resolve(() => {
    listeners.delete(eventName);
  });
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

function emit(eventName: string, payload: Record<string, unknown> = {}) {
  const callback = listeners.get(eventName);
  if (!callback) {
    throw new Error(`No listener registered for ${eventName}`);
  }
  callback({ payload });
}

describe("HistoryPanel", () => {
  beforeEach(() => {
    vi.useRealTimers();
    listeners.clear();
    invokeMock.mockReset();
    listenMock.mockClear();
    useClipboardStore.setState({
      items: [],
      total: 0,
      hasMore: false,
      isLoading: false,
      error: null,
    });

    invokeMock.mockResolvedValue({
      items: [],
      total: 0,
      hasMore: false,
    });
  });

  it("preserves the active search query when clipboard changes trigger a refresh", async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_clipboard_history", {
        limit: 50,
        offset: 0,
        searchQuery: undefined,
      });
    });

    fireEvent.change(screen.getByPlaceholderText("Search clipboard history..."), {
      target: { value: "alpha" },
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenLastCalledWith("get_clipboard_history", {
        limit: 50,
        offset: 0,
        searchQuery: "alpha",
      });
    });

    act(() => {
      emit("clipboard_changed");
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenLastCalledWith("get_clipboard_history", {
        limit: 50,
        offset: 0,
        searchQuery: "alpha",
      });
    });
  });
});
