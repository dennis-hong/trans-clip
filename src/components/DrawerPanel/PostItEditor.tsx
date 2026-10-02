import { useCallback, useEffect, useState, useRef } from "react";
import { Button, Kbd } from "@/components/common";

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

  const handleSave = useCallback(() => {
    const trimmedContent = content.trim();
    if (trimmedContent) {
      onSave(trimmedContent);
    }
  }, [content, onSave]);

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
  }, [handleSave, onClose]);

  const characterCount = content.length;
  const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0;

  const title = mode === "create" ? "새 메모" : "메모 편집";

  return (
    <div className="animate-fade-in fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/25 backdrop-blur-[3px]" onClick={onClose} />

      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="sheet animate-pop-in relative w-full max-w-lg overflow-hidden"
      >
        <h2 className="px-5 pt-4 text-title text-label">{title}</h2>

        <div className="px-5 pb-1 pt-3">
          <textarea
            ref={textareaRef}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="메모 내용을 입력하세요..."
            aria-label={title}
            className="paper h-48 w-full resize-none px-4 py-3 text-reading text-label outline-none [overflow-wrap:anywhere] placeholder:text-label-3"
          />

          <p className="mt-2 text-caption tabular-nums text-label-3">
            {characterCount}자 · {wordCount}단어
          </p>
        </div>

        <div className="flex items-center justify-end gap-2 px-5 pb-4 pt-2">
          <Kbd className="mr-1" aria-hidden="true">
            ESC
          </Kbd>
          <Button onClick={onClose}>취소</Button>
          <Button variant="primary" onClick={handleSave} disabled={!content.trim()}>
            저장
            <span className="text-micro font-medium opacity-70" aria-hidden="true">
              ⌘↵
            </span>
          </Button>
        </div>
      </div>
    </div>
  );
}
