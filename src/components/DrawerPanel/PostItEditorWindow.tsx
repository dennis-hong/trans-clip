import { useCallback, useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button, Kbd } from "@/components/common";

interface EditorParams {
  mode: "create" | "edit";
  itemId?: string;
}

function normalizeEditorParams(params: Partial<EditorParams> | null | undefined): EditorParams {
  const mode = params?.mode === "edit" ? "edit" : "create";
  const itemId = params?.itemId || undefined;

  return {
    mode,
    itemId,
  };
}

function parseUrlParams(): EditorParams {
  const params = new URLSearchParams(window.location.search);
  return normalizeEditorParams({
    mode: params.get("mode") === "edit" ? "edit" : "create",
    itemId: params.get("itemId") || undefined,
  });
}

export function PostItEditorWindow() {
  const [params, setParams] = useState<EditorParams>(() => parseUrlParams());
  const [content, setContent] = useState("");
  const [isLoadingItem, setIsLoadingItem] = useState(
    params.mode === "edit" && Boolean(params.itemId)
  );
  const [isSaving, setIsSaving] = useState(false);
  // Bumped on every "open" request. The load effect depends on it so the memo is
  // reloaded even when the editor is asked for what it already shows (same mode + itemId);
  // otherwise the blanked content and loading state would never be refilled.
  const [openRequest, setOpenRequest] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const closeEditorWindow = useCallback(async () => {
    const currentWindow = getCurrentWindow();

    // Prefer close() so save/cancel always dismisses the editor window.
    try {
      await currentWindow.close();
      return;
    } catch (closeError) {
      console.error("Failed to close editor window, trying hide():", closeError);
    }

    try {
      await currentWindow.hide();
    } catch (hideError) {
      console.error("Failed to hide editor window:", hideError);
    }
  }, []);

  useEffect(() => {
    const unlisten = listen<EditorParams>("postit_editor_open", (event) => {
      const nextParams = normalizeEditorParams(event.payload);
      setParams(nextParams);
      setOpenRequest((request) => request + 1);
      setContent("");
      setIsSaving(false);
      setIsLoadingItem(nextParams.mode === "edit" && Boolean(nextParams.itemId));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (params.mode !== "edit" || !params.itemId) {
      setIsLoadingItem(false);
      return;
    }

    let cancelled = false;

    const loadItem = async () => {
      try {
        const item = await invoke<{ content: string }>("get_clipboard_item", {
          id: params.itemId,
        });
        if (!cancelled) {
          setContent(item.content ?? "");
        }
      } catch (error) {
        console.error("Failed to load item for editing:", error);
      } finally {
        if (!cancelled) {
          setIsLoadingItem(false);
        }
      }
    };

    void loadItem();

    return () => {
      cancelled = true;
    };
  }, [params.itemId, params.mode, openRequest]);

  // Focus textarea on mount
  useEffect(() => {
    if (!isLoadingItem) {
      textareaRef.current?.focus();
    }
  }, [isLoadingItem]);

  const handleSave = useCallback(async () => {
    const trimmedContent = content.trim();
    if (!trimmedContent || isSaving) return;

    setIsSaving(true);

    try {
      if (params.mode === "edit" && params.itemId) {
        await invoke("update_clipboard_item", {
          id: params.itemId,
          content: trimmedContent,
        });
      } else {
        await invoke("create_clipboard_item", {
          content: trimmedContent,
        });
      }

      // Emit event to notify main window
      await emit("postit_saved", { mode: params.mode, itemId: params.itemId });
    } catch (error) {
      console.error("Failed to save:", error);
      setIsSaving(false);
      return;
    }

    await closeEditorWindow();
  }, [content, params, isSaving, closeEditorWindow]);

  const handleClose = useCallback(async () => {
    setIsSaving(false);
    await closeEditorWindow();
  }, [closeEditorWindow]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // ESC to close
      if (e.key === "Escape") {
        e.preventDefault();
        handleClose();
        return;
      }

      // Cmd+Enter to save
      if (e.key === "Enter" && e.metaKey) {
        e.preventDefault();
        handleSave();
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleClose, handleSave]);

  const characterCount = content.length;
  const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0;

  const title = params.mode === "create" ? "새 메모" : "메모 편집";

  // Keep the native title bar in sync with the mode (the window is reused).
  useEffect(() => {
    void getCurrentWindow()
      .setTitle(title)
      .catch((error) => {
        console.error("Failed to update editor window title:", error);
      });
  }, [title]);

  return (
    <div className="flex h-screen flex-col bg-[var(--window-bg)]">
      {/* The native title bar already names the window; the page is just the memo. */}
      <div className="min-h-0 flex-1 px-4 pb-2 pt-4">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="메모 내용을 입력하세요..."
          aria-label={title}
          disabled={isLoadingItem || isSaving}
          className="paper h-full w-full resize-none px-5 py-4 text-[15px] leading-[1.75] text-label outline-none [overflow-wrap:anywhere] placeholder:text-label-3"
        />
      </div>

      <footer className="flex shrink-0 items-center justify-between gap-3 px-4 pb-3.5 pt-1.5">
        <span className="text-caption tabular-nums text-label-3">
          {characterCount}자 · {wordCount}단어
        </span>

        <div className="flex items-center gap-2">
          <Kbd className="mr-1" aria-hidden="true">
            ESC
          </Kbd>
          <Button onClick={handleClose}>취소</Button>
          <Button
            variant="primary"
            onClick={handleSave}
            disabled={!content.trim() || isSaving || isLoadingItem}
            loading={isSaving}
            title="저장 (⌘+Enter)"
          >
            {isSaving ? "저장 중..." : "저장"}
            {!isSaving && (
              <span className="text-micro font-medium opacity-70" aria-hidden="true">
                ⌘↵
              </span>
            )}
          </Button>
        </div>
      </footer>
    </div>
  );
}
