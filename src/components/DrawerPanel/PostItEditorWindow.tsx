import { useCallback, useEffect, useState, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface EditorParams {
  mode: "create" | "edit";
  itemId?: string;
  initialContent: string;
}

function parseUrlParams(): EditorParams {
  const params = new URLSearchParams(window.location.search);
  const mode = params.get("mode") as "create" | "edit" || "create";
  const itemId = params.get("itemId") || undefined;
  const content = params.get("content") || "";

  return {
    mode,
    itemId,
    initialContent: decodeURIComponent(content),
  };
}

export function PostItEditorWindow() {
  const params = useMemo(() => parseUrlParams(), []);
  const [content, setContent] = useState(params.initialContent);
  const [isSaving, setIsSaving] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Focus textarea on mount
  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

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

      // Close this window after a small delay to allow WebKit to finish processing
      // This prevents a race condition that can cause WebKit crashes
      const currentWindow = getCurrentWindow();
      await new Promise((resolve) => setTimeout(resolve, 50));
      await currentWindow.close();
    } catch (error) {
      console.error("Failed to save:", error);
      setIsSaving(false);
    }
  }, [content, params, isSaving]);

  const handleClose = useCallback(async () => {
    const currentWindow = getCurrentWindow();
    // Small delay to allow WebKit to finish processing before window destruction
    await new Promise((resolve) => setTimeout(resolve, 50));
    await currentWindow.close();
  }, []);

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

  return (
    <div className="flex flex-col h-screen bg-white">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center gap-2">
          <svg
            className="w-5 h-5 text-amber-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
            />
          </svg>
          <span className="font-medium text-gray-800">{title}</span>
        </div>
        <button
          onClick={handleClose}
          className="p-1.5 rounded-lg hover:bg-gray-200 transition-colors"
          title="닫기 (ESC)"
        >
          <svg
            className="w-4 h-4 text-gray-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 p-4 flex flex-col">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="메모 내용을 입력하세요..."
          className="flex-1 w-full p-4 bg-yellow-50 border-2 border-yellow-200 rounded-lg resize-none text-sm text-gray-800 leading-relaxed focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-amber-400 placeholder:text-gray-400"
        />

        {/* Character/Word count */}
        <div className="flex items-center justify-between mt-2 text-xs text-gray-500">
          <span>
            {characterCount}자 · {wordCount}단어
          </span>
          <div className="flex items-center gap-2">
            <span className="font-mono bg-gray-100 px-1.5 py-0.5 rounded">
              ESC
            </span>
            <span>취소</span>
            <span className="mx-1">·</span>
            <span className="font-mono bg-gray-100 px-1.5 py-0.5 rounded">
              ⌘↵
            </span>
            <span>저장</span>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-gray-200 bg-gray-50">
        <button
          onClick={handleClose}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
        >
          취소
        </button>
        <button
          onClick={handleSave}
          disabled={!content.trim() || isSaving}
          className="px-4 py-2 text-sm font-medium text-white bg-amber-500 rounded-lg hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isSaving ? "저장 중..." : "저장"}
        </button>
      </div>
    </div>
  );
}
