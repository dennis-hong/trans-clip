import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore } from "./clipboardStore";
import type { ClipboardItem } from "@/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function makeItem(id: string, content: string): ClipboardItem {
  return {
    id,
    content,
    contentPreview: content,
    copiedAt: new Date().toISOString(),
    isPinned: false,
  };
}

describe("useClipboardStore", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    useClipboardStore.setState({
      items: [],
      total: 0,
      hasMore: false,
      isLoading: false,
      error: null,
    });
  });

  it("fetchHistory replaces items on fresh load", async () => {
    const items = [makeItem("1", "hello"), makeItem("2", "world")];
    invokeMock.mockResolvedValueOnce({
      items,
      total: 2,
      hasMore: false,
    });

    await useClipboardStore.getState().fetchHistory();

    const state = useClipboardStore.getState();
    expect(state.items).toEqual(items);
    expect(state.total).toBe(2);
    expect(state.hasMore).toBe(false);
    expect(state.isLoading).toBe(false);
  });

  it("fetchHistory appends items when offset is provided", async () => {
    useClipboardStore.setState({
      items: [makeItem("1", "first")],
      total: 1,
      hasMore: true,
      isLoading: false,
      error: null,
    });
    invokeMock.mockResolvedValueOnce({
      items: [makeItem("2", "second")],
      total: 2,
      hasMore: false,
    });

    await useClipboardStore.getState().fetchHistory({ offset: 1 });

    const state = useClipboardStore.getState();
    expect(state.items.map((item) => item.id)).toEqual(["1", "2"]);
    expect(state.total).toBe(2);
    expect(state.hasMore).toBe(false);
  });

  it("fetchHistory ignores stale responses when calls overlap", async () => {
    let resolveFirst: ((value: unknown) => void) | undefined;
    let resolveSecond: ((value: unknown) => void) | undefined;

    invokeMock.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveFirst = resolve;
        })
    );
    invokeMock.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveSecond = resolve;
        })
    );

    const firstRequest = useClipboardStore.getState().fetchHistory({ searchQuery: "old" });
    const secondRequest = useClipboardStore.getState().fetchHistory({ searchQuery: "new" });

    resolveSecond?.({
      items: [makeItem("2", "new-result")],
      total: 1,
      hasMore: false,
    });
    await secondRequest;

    resolveFirst?.({
      items: [makeItem("1", "old-result")],
      total: 1,
      hasMore: false,
    });
    await firstRequest;

    const state = useClipboardStore.getState();
    expect(state.items.map((item) => item.id)).toEqual(["2"]);
    expect(state.items[0]?.content).toBe("new-result");
  });

  it("addItem deduplicates by content and moves latest to top", () => {
    useClipboardStore.setState({
      items: [makeItem("1", "same"), makeItem("2", "other")],
      total: 2,
      hasMore: false,
      isLoading: false,
      error: null,
    });

    useClipboardStore.getState().addItem(makeItem("3", "same"));
    const state = useClipboardStore.getState();

    expect(state.items.map((item) => item.id)).toEqual(["3", "2"]);
    expect(state.total).toBe(2);
  });

  it("deleteItem removes row and decrements total on success", async () => {
    useClipboardStore.setState({
      items: [makeItem("1", "a"), makeItem("2", "b")],
      total: 2,
      hasMore: false,
      isLoading: false,
      error: null,
    });
    invokeMock.mockResolvedValueOnce({ success: true, error: null });

    const ok = await useClipboardStore.getState().deleteItem("1");

    const state = useClipboardStore.getState();
    expect(ok).toBe(true);
    expect(state.items.map((item) => item.id)).toEqual(["2"]);
    expect(state.total).toBe(1);
  });
});
