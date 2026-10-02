import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PostItEditorWindow } from "./PostItEditorWindow";

type OpenPayload = { mode: "create" | "edit"; itemId?: string };

const invokeMock = vi.fn();
let openHandler: ((event: { payload: OpenPayload }) => void) | null = null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(() => Promise.resolve()),
  listen: vi.fn((eventName: string, handler: (event: { payload: OpenPayload }) => void) => {
    if (eventName === "postit_editor_open") {
      openHandler = handler;
    }
    return Promise.resolve(() => {
      if (eventName === "postit_editor_open") {
        openHandler = null;
      }
    });
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    close: vi.fn(() => Promise.resolve()),
    hide: vi.fn(() => Promise.resolve()),
    setTitle: vi.fn(() => Promise.resolve()),
  }),
}));

const MEMOS: Record<string, string> = {
  "memo-1": "first memo",
  "memo-2": "second memo",
};

function loadCalls(itemId?: string) {
  return invokeMock.mock.calls.filter(
    ([command, args]) =>
      command === "get_clipboard_item" && (itemId === undefined || args?.id === itemId)
  );
}

/** What the backend does when the editor window already exists. */
function reopenEditor(payload: OpenPayload) {
  act(() => {
    openHandler?.({ payload });
  });
}

describe("PostItEditorWindow", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openHandler = null;
    invokeMock.mockImplementation((command: string, args?: { id?: string }) => {
      if (command === "get_clipboard_item") {
        return Promise.resolve({ content: MEMOS[args?.id ?? ""] ?? "" });
      }
      return Promise.resolve(null);
    });
    window.history.pushState({}, "", "/?window=editor&mode=edit&itemId=memo-1");
  });

  async function renderLoadedEditor() {
    render(<PostItEditorWindow />);
    const textarea = (await screen.findByLabelText("메모 편집")) as HTMLTextAreaElement;
    await waitFor(() => expect(textarea).toHaveValue("first memo"));
    await waitFor(() => expect(openHandler).not.toBeNull());
    return textarea;
  }

  it("loads the memo being edited", async () => {
    const textarea = await renderLoadedEditor();

    expect(textarea).toBeEnabled();
    expect(loadCalls("memo-1")).toHaveLength(1);
  });

  it("reloads the memo when the open editor is asked for the same memo again", async () => {
    const textarea = await renderLoadedEditor();

    reopenEditor({ mode: "edit", itemId: "memo-1" });

    await waitFor(() => expect(textarea).toBeEnabled());
    await waitFor(() => expect(textarea).toHaveValue("first memo"));
    expect(loadCalls("memo-1")).toHaveLength(2);
  });

  it("loads another memo when the open editor is asked for a different one", async () => {
    const textarea = await renderLoadedEditor();

    reopenEditor({ mode: "edit", itemId: "memo-2" });

    await waitFor(() => expect(textarea).toHaveValue("second memo"));
    expect(textarea).toBeEnabled();
  });

  it("starts an empty draft when the open editor is asked to create a memo", async () => {
    const textarea = await renderLoadedEditor();

    reopenEditor({ mode: "create" });

    await waitFor(() => expect(textarea).toHaveValue(""));
    expect(textarea).toBeEnabled();
    expect(screen.getByLabelText("새 메모")).toBe(textarea);
  });
});
