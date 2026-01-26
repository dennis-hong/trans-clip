import { useCallback, useEffect, useState, useRef } from "react";

interface PostItEditorProps {
  mode: "create" | "edit";
  initialContent?: string;
  itemId?: string;
  onSave: (content: string) => void;
  onClose: () => void;
}

export function PostItEditor({
  mode,
  initialContent = "",
  onSave,
  onClose,
}: PostItEditorProps) {
  const [content, setContent] = useState(initialContent);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Focus textarea on mount
  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // ESC to close
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
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
  }, [content, onClose]);

  const handleSave = useCallback(() => {
    const trimmedContent = content.trim();
    if (trimmedContent) {
      onSave(trimmedContent);
    }
  }, [content, onSave]);

  const characterCount = content.length;
  const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0;

  const title = mode === "create" ? "새 메모" : "메모 편집";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="w-full max-w-lg mx-4 bg-white rounded-xl shadow-2xl overflow-hidden">
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
            onClick={onClose}
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
        <div className="p-4">
          <textarea
            ref={textareaRef}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="메모 내용을 입력하세요..."
            className="w-full h-48 p-4 bg-yellow-50 border-2 border-yellow-200 rounded-lg resize-none text-sm text-gray-800 leading-relaxed focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-amber-400 placeholder:text-gray-400"
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
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
          >
            취소
          </button>
          <button
            onClick={handleSave}
            disabled={!content.trim()}
            className="px-4 py-2 text-sm font-medium text-white bg-amber-500 rounded-lg hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            저장
          </button>
        </div>
      </div>
    </div>
  );
}
