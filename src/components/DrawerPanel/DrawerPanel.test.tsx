import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DrawerPanel } from "./DrawerPanel";
import { useClipboardStore } from "@/store";
import type { ClipboardItem } from "@/types";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    outerSize: () => Promise.resolve({ width: 2400, height: 560 }),
    scaleFactor: () => Promise.resolve(2),
    onResized: () => Promise.resolve(() => {}),
  }),
}));

// The panel hosts these views, but this suite only exercises the history strip.
vi.mock("@/components/Settings/SettingsPanel", () => ({ SettingsPanel: () => null }));
vi.mock("@/components/GlossaryManager/GlossaryList", () => ({ GlossaryList: () => null }));

function makeItem(id: string, content: string, isPinned = false): ClipboardItem {
  return {
    id,
    content,
    contentPreview: content,
    copiedAt: new Date().toISOString(),
    isPinned,
    metadata: { characterCount: content.length, wordCount: 1 },
  };
}

const ITEMS = [makeItem("a", "first note"), makeItem("b", "second note")];

function setClipboardCalls() {
  return invokeMock.mock.calls.filter(([command]) => command === "set_clipboard");
}

async function renderPanel(props: Partial<React.ComponentProps<typeof DrawerPanel>> = {}) {
  render(<DrawerPanel isStealthMode onClose={vi.fn()} {...props} />);
  // Wait for the mocked history to land in the strip.
  await screen.findByRole("button", { name: "메모 first note 복사" });
  return screen.getByLabelText("클립보드 히스토리 검색") as HTMLInputElement;
}

describe("DrawerPanel keyboard shortcuts", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "get_clipboard_history":
          return Promise.resolve({ items: ITEMS, total: ITEMS.length, hasMore: false });
        case "get_monitors":
          return Promise.resolve([]);
        case "get_current_monitor_index":
          return Promise.resolve(0);
        default:
          return Promise.resolve(null);
      }
    });
    useClipboardStore.setState({ items: [], total: 0, hasMore: false, isLoading: false, error: null });
    // jsdom does not implement element scrolling.
    Element.prototype.scrollBy = vi.fn();
  });

  it("copies the Nth note when a number key is pressed", async () => {
    await renderPanel();

    fireEvent.keyDown(window, { key: "2", code: "Digit2" });

    expect(setClipboardCalls()).toEqual([["set_clipboard", { text: "second note" }]]);
  });

  it("scrolls the strip with the arrow keys", async () => {
    await renderPanel();

    const notPrevented = fireEvent.keyDown(window, { key: "ArrowRight", code: "ArrowRight" });

    expect(notPrevented).toBe(false);
    expect(Element.prototype.scrollBy).toHaveBeenCalledTimes(1);
  });

  it("leaves digits to the search field instead of quick-copying", async () => {
    const search = await renderPanel();
    search.focus();

    const notPrevented = fireEvent.keyDown(search, { key: "1", code: "Digit1" });

    expect(notPrevented).toBe(true);
    expect(setClipboardCalls()).toHaveLength(0);
  });

  it("leaves the arrow keys to the search field's caret", async () => {
    const search = await renderPanel();
    search.focus();

    const notPrevented = fireEvent.keyDown(search, { key: "ArrowLeft", code: "ArrowLeft" });

    expect(notPrevented).toBe(true);
    expect(Element.prototype.scrollBy).not.toHaveBeenCalled();
  });

  it("does not treat Shift+digit typed in the search field as translate", async () => {
    const onTranslate = vi.fn();
    const search = await renderPanel({ onTranslate });
    search.focus();

    // Shift+1 types "!" in the search field.
    const notPrevented = fireEvent.keyDown(search, { key: "!", code: "Digit1", shiftKey: true });

    expect(notPrevented).toBe(true);
    expect(onTranslate).not.toHaveBeenCalled();
  });

  it("still translates the Nth note with Shift+digit outside of text fields", async () => {
    const onTranslate = vi.fn();
    await renderPanel({ onTranslate });

    fireEvent.keyDown(window, { key: "!", code: "Digit1", shiftKey: true });

    expect(onTranslate).toHaveBeenCalledWith("first note");
  });

  it("keeps modifier shortcuts working while the search field is focused", async () => {
    const onPolish = vi.fn();
    const search = await renderPanel({ onPolish });
    search.focus();

    fireEvent.keyDown(search, { key: "1", code: "Digit1", ctrlKey: true });

    expect(onPolish).toHaveBeenCalledWith("first note");
  });
});
