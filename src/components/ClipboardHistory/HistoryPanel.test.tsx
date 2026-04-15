import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HistoryPanel } from "./HistoryPanel";

const fetchHistoryMock = vi.fn();
const listeners = new Map<string, () => void>();

vi.mock("@/store", () => ({
  useClipboardStore: () => ({
    items: [],
    isLoading: false,
    error: null,
    hasMore: false,
    fetchHistory: fetchHistoryMock,
    deleteItem: vi.fn(),
    togglePin: vi.fn(),
    clearAll: vi.fn(),
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: () => void) => {
    listeners.set(eventName, callback);
    return Promise.resolve(() => {
      listeners.delete(eventName);
    });
  }),
}));

describe("HistoryPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    fetchHistoryMock.mockReset();
    listeners.clear();
  });

  it("preserves the active search query when clipboard refresh events arrive", async () => {
    render(<HistoryPanel />);

    const searchInput = screen.getByPlaceholderText("Search clipboard history...");
    fireEvent.change(searchInput, { target: { value: "urgent" } });

    expect(fetchHistoryMock).toHaveBeenNthCalledWith(1);
    expect(fetchHistoryMock).toHaveBeenNthCalledWith(2, { searchQuery: "urgent" });

    act(() => {
      listeners.get("clipboard_changed")?.();
      vi.advanceTimersByTime(100);
    });

    expect(fetchHistoryMock).toHaveBeenNthCalledWith(3, { searchQuery: "urgent" });
  });
});
