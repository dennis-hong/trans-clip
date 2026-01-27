import { useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslationStream } from "@/hooks/useTranslationStream";
import { useClipboardStore } from "@/store";
import { CLAUDE_MODELS, type ClaudeModel } from "@/types";

interface TranslationPopupProps {
  sourceText: string;
  onClose: () => void;
}

export function TranslationPopup({
  sourceText,
  onClose,
}: TranslationPopupProps) {
  const {
    translate,
    isStreaming,
    streamedText,
    fullText,
    detectedLanguage,
    fromCache,
    glossaryApplied,
    error,
  } = useTranslationStream();
  const { createItem } = useClipboardStore();
  const [editableText, setEditableText] = useState(sourceText);
  const [isSaved, setIsSaved] = useState(false);
  const [selectedModel, setSelectedModel] = useState<ClaudeModel | undefined>(undefined);
  // Sync editableText when sourceText changes
  useEffect(() => {
    setEditableText(sourceText);
  }, [sourceText]);

  // Translate on mount only
  useEffect(() => {
    if (sourceText) {
      translate(sourceText);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceText]);  // Intentionally exclude translate to prevent re-runs

  const handleCopy = useCallback(async () => {
    const textToCopy = fullText || streamedText;
    if (textToCopy) {
      try {
        await invoke("set_clipboard", { text: textToCopy });
        onClose();
      } catch (err) {
        console.error("Failed to copy:", err);
      }
    }
  }, [fullText, streamedText, onClose]);

  const handleReplace = useCallback(async () => {
    const textToReplace = fullText || streamedText;
    if (textToReplace) {
      try {
        // First hide the popup to return focus to the original app
        await invoke("hide_translation_popup");
        // Give the original app time to regain focus (increased delay)
        await new Promise(resolve => setTimeout(resolve, 300));
        // Now simulate paste in the original app
        const response = await invoke<{ success: boolean; error?: { code: string; message: string } }>("paste_text", { text: textToReplace });
        console.log("paste_text response:", response);
        if (!response.success && response.error) {
          console.error("Paste failed:", response.error.code, response.error.message);
        }
        onClose();
      } catch (err) {
        console.error("Failed to replace:", err);
      }
    }
  }, [fullText, streamedText, onClose]);

  // Handle keyboard shortcuts (ESC to close, Cmd/Ctrl+Enter to replace)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        if (!isStreaming && (fullText || streamedText)) {
          handleReplace();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, isStreaming, fullText, streamedText, handleReplace]);

  const handleRetranslate = useCallback(() => {
    if (editableText.trim()) {
      translate(editableText, selectedModel);
    }
  }, [editableText, selectedModel, translate]);

  const handleSaveAsPostIt = useCallback(async () => {
    const textToSave = fullText || streamedText;
    if (textToSave) {
      const newItem = await createItem(textToSave);
      if (newItem) {
        setIsSaved(true);
        // Reset saved state after 2 seconds
        setTimeout(() => setIsSaved(false), 2000);
      }
    }
  }, [fullText, streamedText, createItem]);

  // Check if source text has been modified
  const isSourceModified = editableText !== sourceText;

  const displayLanguage = detectedLanguage ?? "en";
  const targetLanguage = displayLanguage === "ko" ? "en" : "ko";
  const sourceLabel = displayLanguage === "ko" ? "한국어" : "English";
  const targetLabel = targetLanguage === "ko" ? "한국어" : "English";

  // Determine if we have content to display
  const hasResult = Boolean(fullText || streamedText);

  return (
    <div className="flex flex-col h-full w-full">
      {/* Header - DrawerPanel과 동일한 스타일 */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-gray-200/50">
        {/* Back button */}
        <button
          onClick={onClose}
          className="p-1 rounded-lg hover:bg-gray-200/80 transition-colors"
          title="뒤로"
        >
          <svg className="w-5 h-5 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>

        {/* Title */}
        <div className="flex items-center gap-2">
          <svg className="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
            />
          </svg>
          <span className="font-medium text-gray-800">번역</span>
        </div>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Footer info */}
        <div className="flex items-center gap-3 text-xs text-gray-500">
          {glossaryApplied && glossaryApplied.length > 0 && (
            <span className="flex items-center gap-1">
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
              </svg>
              용어집 {glossaryApplied.length}개 적용
            </span>
          )}
          {fromCache && (
            <span className="text-green-600">캐시</span>
          )}
        </div>

        {/* Keyboard hint */}
        <div className="hidden sm:flex items-center gap-1 text-[10px] text-gray-400">
          <span className="font-mono bg-gray-100 px-1.5 py-0.5 rounded">ESC</span>
        </div>

        {/* Close button */}
        <button
          onClick={onClose}
          className="p-1.5 rounded-lg hover:bg-gray-200/80 transition-colors"
          title="닫기 (ESC)"
        >
          <svg
            className="w-4 h-4 text-gray-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Content - 포스트잇 스타일 카드 */}
      <div className="flex-1 overflow-hidden p-4">
        <div className="h-full flex gap-4">
          {/* Source Text - 노란색 포스트잇 (수정 가능) */}
          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center gap-2 mb-2 px-1">
              <span className="text-xs font-medium text-amber-700">📝 원문</span>
              <span className="text-[10px] text-amber-600 bg-amber-100 px-1.5 py-0.5 rounded">
                {sourceLabel}
              </span>
              {isSourceModified && (
                <span className="text-[10px] text-orange-600 bg-orange-100 px-1.5 py-0.5 rounded">
                  수정됨
                </span>
              )}
            </div>
            <textarea
              value={editableText}
              onChange={(e) => setEditableText(e.target.value)}
              className="flex-1 p-4 bg-yellow-100 border-2 border-yellow-300 rounded-lg shadow-md resize-none text-sm text-gray-800 leading-relaxed focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-amber-400"
              placeholder="원문을 수정하여 다시 번역할 수 있습니다..."
            />
          </div>

          {/* Arrow */}
          <div className="flex items-center justify-center px-2 text-gray-400">
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
            </svg>
          </div>

          {/* Translated Text - 파란색 포스트잇 */}
          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center gap-2 mb-2 px-1">
              <span className="text-xs font-medium text-blue-700">✨ 번역 결과</span>
              <span className="text-[10px] text-blue-600 bg-blue-100 px-1.5 py-0.5 rounded">
                {targetLabel}
              </span>
            </div>
            <div className="flex-1 p-4 bg-blue-100 border-2 border-blue-300 rounded-lg shadow-md overflow-y-auto">
              {error ? (
                <p className="text-sm text-red-600">{error}</p>
              ) : (fullText || streamedText) ? (
                <p className="text-sm text-gray-800 whitespace-pre-wrap break-words leading-relaxed">
                  {fullText || streamedText}
                  {isStreaming && (
                    <span className="inline-block w-0.5 h-4 ml-0.5 bg-blue-600 animate-pulse" />
                  )}
                </p>
              ) : (
                <div className="flex items-center justify-center h-full">
                  <div className="flex items-center gap-2 text-sm text-blue-600">
                    <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                      />
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      />
                    </svg>
                    <span>번역 중...</span>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Footer - 액션 버튼 */}
      <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-gray-200/50">
        {/* Model selector + Retranslate button group */}
        <div className="flex items-center gap-2 mr-auto">
          <label className="text-[10px] text-gray-500">모델</label>
          <select
            value={selectedModel ?? ""}
            onChange={(e) => setSelectedModel(e.target.value ? e.target.value as ClaudeModel : undefined)}
            className="px-2 py-1 text-xs bg-white border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="">기본값</option>
            {CLAUDE_MODELS.map((model) => (
              <option key={model.id} value={model.id}>
                {model.name} ({model.description})
              </option>
            ))}
          </select>
          <button
            onClick={handleRetranslate}
            disabled={isStreaming || !editableText.trim()}
            className="px-3 py-1 text-xs font-medium text-blue-700 bg-blue-50 border border-blue-300 rounded-md hover:bg-blue-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            다시 번역
          </button>
        </div>
        <button
          onClick={handleSaveAsPostIt}
          disabled={isStreaming || !hasResult || isSaved}
          className={`px-4 py-2 text-sm font-medium border rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
            isSaved
              ? "text-green-700 bg-green-50 border-green-300"
              : "text-amber-700 bg-white border-amber-300 hover:bg-amber-50"
          }`}
        >
          {isSaved ? "저장됨!" : "메모로 저장"}
        </button>
        <button
          onClick={handleCopy}
          disabled={isStreaming || !hasResult}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          복사
        </button>
        <button
          onClick={handleReplace}
          disabled={isStreaming || !hasResult}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          title="바꾸기 (⌘+Enter)"
        >
          바꾸기
          <span className="ml-1.5 text-[10px] opacity-70">⌘↵</span>
        </button>
      </div>
    </div>
  );
}
