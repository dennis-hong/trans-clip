import { useEffect, useCallback, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { usePolishStream } from "@/hooks/usePolishStream";
import { useWindowDrag } from "@/hooks/useWindowDrag";
import {
  usePolishStore,
  useClipboardStore,
  useSettingsStore,
  POLISH_CONTEXTS,
  POLISH_CHANNELS,
  POLISH_OPTIONS,
} from "@/store";
import type { ModelProfileId, PolishContext, PolishChannel, PolishOption } from "@/types";
import { DEFAULT_MODEL_PROFILE_ID, formatModelProfileOption } from "@/types";

interface PolishPopupProps {
  sourceText: string;
  onClose: () => void;
  onTranslate?: (text: string) => void;
  onPreferredHeightChange?: (height: number) => void;
}

export function PolishPopup({
  sourceText,
  onClose,
  onTranslate,
  onPreferredHeightChange,
}: PolishPopupProps) {
  const {
    polish,
    isStreaming,
    streamedText,
    fullText,
    error,
  } = usePolishStream();
  const { createItem } = useClipboardStore();
  const { settings, fetchSettings } = useSettingsStore();
  const { handleDragStart } = useWindowDrag();
  const [editableText, setEditableText] = useState(sourceText);
  const [isSaved, setIsSaved] = useState(false);
  const [selectedModel, setSelectedModel] = useState<ModelProfileId | undefined>(undefined);
  const saveStatusTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heightMeasureTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastSuggestedHeightRef = useRef<number | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const sourceTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const resultContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    return () => {
      if (saveStatusTimeoutRef.current) {
        clearTimeout(saveStatusTimeoutRef.current);
      }
      if (heightMeasureTimeoutRef.current) {
        clearTimeout(heightMeasureTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!settings) {
      void fetchSettings();
    }
  }, [fetchSettings, settings]);

  // Sync editableText when sourceText changes
  useEffect(() => {
    setEditableText(sourceText);
  }, [sourceText]);

  const modelProfiles = settings?.aiModelProfiles ?? [];
  const providerConfigs = settings?.aiProviderConfigs ?? [];
  const defaultModel = settings?.preferredModelProfileId ?? DEFAULT_MODEL_PROFILE_ID;
  const displayModel = selectedModel ?? defaultModel;

  useEffect(() => {
    if (selectedModel === defaultModel) {
      setSelectedModel(undefined);
    }
  }, [defaultModel, selectedModel]);

  // Get last used settings from store
  const {
    lastContext,
    lastChannel,
    lastOptions,
    setLastContext,
    setLastChannel,
    toggleOption,
  } = usePolishStore();

  // Polish on source text changes (including first mount)
  useEffect(() => {
    if (sourceText) {
      polish(sourceText, lastContext, lastChannel, lastOptions);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceText]);

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
        // Run hide + paste in one backend command to avoid a race where the
        // second invoke is dropped after hiding the WebView.
        const response = await invoke<{
          success: boolean;
          error?: { code: string; message: string };
        }>("hide_and_paste_text", { text: textToReplace });
        if (!response.success && response.error) {
          console.error(
            "Paste failed:",
            response.error.code,
            response.error.message
          );
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

  const handleRepolish = useCallback(() => {
    if (editableText.trim()) {
      polish(editableText, lastContext, lastChannel, lastOptions, selectedModel);
    }
  }, [editableText, lastContext, lastChannel, lastOptions, selectedModel, polish]);

  const handleModelChange = (model: ModelProfileId) => {
    setSelectedModel(model === defaultModel ? undefined : model);
  };

  const handleSaveAsPostIt = useCallback(async () => {
    const textToSave = fullText || streamedText;
    if (textToSave) {
      const newItem = await createItem(textToSave);
      if (newItem) {
        setIsSaved(true);
        // Reset saved state after 2 seconds
        if (saveStatusTimeoutRef.current) {
          clearTimeout(saveStatusTimeoutRef.current);
        }
        saveStatusTimeoutRef.current = setTimeout(() => {
          setIsSaved(false);
          saveStatusTimeoutRef.current = null;
        }, 2000);
      }
    }
  }, [fullText, streamedText, createItem]);

  const handleTranslate = useCallback(() => {
    const textToTranslate = fullText || streamedText;
    if (textToTranslate && onTranslate) {
      onTranslate(textToTranslate);
    }
  }, [fullText, streamedText, onTranslate]);

  // Check if source text has been modified
  const isSourceModified = editableText !== sourceText;

  // Determine if we have content to display
  const hasResult = Boolean(fullText || streamedText);
  const resultStatusMessage = error
    ? "글 다듬기에 문제가 생겼습니다."
    : isStreaming
      ? "글을 다듬는 중입니다."
      : hasResult
        ? "글 다듬기가 완료되었습니다."
        : "글 다듬기를 준비 중입니다.";

  const schedulePreferredHeightUpdate = useCallback(() => {
    if (!onPreferredHeightChange) {
      return;
    }

    if (heightMeasureTimeoutRef.current) {
      clearTimeout(heightMeasureTimeoutRef.current);
    }

    heightMeasureTimeoutRef.current = setTimeout(() => {
      const containerEl = containerRef.current;
      const sourceEl = sourceTextareaRef.current;
      const resultEl = resultContainerRef.current;

      if (!containerEl || !sourceEl || !resultEl) {
        return;
      }

      const visibleContentHeight = Math.max(
        sourceEl.clientHeight,
        resultEl.clientHeight,
        1
      );
      const baseChromeHeight = Math.max(containerEl.clientHeight - visibleContentHeight, 0);
      const desiredContentHeight = Math.max(
        sourceEl.scrollHeight,
        resultEl.scrollHeight,
        180
      );

      let suggestedHeight = Math.round(baseChromeHeight + desiredContentHeight + 20);
      if (isStreaming && lastSuggestedHeightRef.current !== null) {
        suggestedHeight = Math.max(suggestedHeight, lastSuggestedHeightRef.current);
      }

      if (
        lastSuggestedHeightRef.current === null
        || Math.abs(suggestedHeight - lastSuggestedHeightRef.current) >= 24
      ) {
        lastSuggestedHeightRef.current = suggestedHeight;
        onPreferredHeightChange(suggestedHeight);
      }
    }, 120);
  }, [isStreaming, onPreferredHeightChange]);

  const handleContextChange = (context: PolishContext) => {
    setLastContext(context);
  };

  const handleChannelChange = (channel: PolishChannel) => {
    setLastChannel(channel);
  };

  const handleOptionToggle = (option: PolishOption) => {
    toggleOption(option);
  };

  useEffect(() => {
    schedulePreferredHeightUpdate();
  }, [
    editableText,
    fullText,
    streamedText,
    error,
    isStreaming,
    lastContext,
    lastChannel,
    lastOptions,
    selectedModel,
    schedulePreferredHeightUpdate,
  ]);

  return (
    <div ref={containerRef} className="flex flex-col h-full w-full">
      {/* Header - Draggable area */}
      <div
        className="flex items-center gap-3 px-4 py-2 cursor-move select-none border-b border-gray-200/50"
        onMouseDown={handleDragStart}
      >
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
          <svg className="w-5 h-5 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
            />
          </svg>
          <span className="font-medium text-gray-800">글 다듬기</span>
        </div>

        {/* Spacer */}
        <div className="flex-1" />

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

      {/* Content - 상단: 원문/결과 좌우, 하단: 설정 */}
      <div className="flex-1 overflow-hidden p-4 flex flex-col gap-3">
        {/* Top: Source & Result 좌우 배치 */}
        <div className="flex-1 flex gap-4 min-h-0">
          {/* Source Text - 노란색 포스트잇 (수정 가능) */}
          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center gap-2 mb-2 px-1">
              <span className="text-xs font-medium text-amber-700">📝 원문 (러프한 초안)</span>
              {isSourceModified && (
                <span className="text-[10px] text-orange-600 bg-orange-100 px-1.5 py-0.5 rounded">
                  수정됨
                </span>
              )}
            </div>
            <textarea
              ref={sourceTextareaRef}
              value={editableText}
              onChange={(e) => setEditableText(e.target.value)}
              className="flex-1 p-3 bg-yellow-100 border-2 border-yellow-300 rounded-lg shadow-md resize-none text-sm text-gray-800 leading-relaxed focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-amber-400"
              placeholder="원문을 수정하여 다시 다듬을 수 있습니다..."
            />
          </div>

          {/* Arrow */}
          <div className="flex items-center justify-center px-2 text-gray-400">
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
            </svg>
          </div>

          {/* Result - 녹색 포스트잇 */}
          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center gap-2 mb-2 px-1">
              <span className="text-xs font-medium text-green-700">✨ 정돈된 결과</span>
            </div>
            <div
              ref={resultContainerRef}
              className="flex-1 p-3 bg-green-100 border-2 border-green-300 rounded-lg shadow-md overflow-y-auto"
              role="status"
              aria-live="polite"
              aria-atomic="true"
              aria-label="다듬기 결과"
            >
              <span className="sr-only">{resultStatusMessage}</span>
              {error ? (
                <div>
                  <p className="text-sm text-red-700">
                    글 다듬기에 문제가 생겼습니다. 다시 시도해 주세요.
                  </p>
                  <p className="mt-1 text-xs text-red-600 break-words">{error}</p>
                </div>
              ) : (fullText || streamedText) ? (
                <p className="text-sm text-gray-800 whitespace-pre-wrap break-words leading-relaxed">
                  {fullText || streamedText}
                  {isStreaming && (
                    <span className="inline-block w-0.5 h-4 ml-0.5 bg-green-600 animate-pulse" />
                  )}
                </p>
              ) : (
                <div className="flex items-center justify-center h-full">
                  <div className="flex items-center gap-2 text-sm text-green-600">
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
                    <span>다듬는 중...</span>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Bottom: Settings - 가로로 길게 */}
        <div className="flex-shrink-0 p-3 bg-gray-100 border-2 border-gray-300 rounded-lg shadow-md">
          <div className="flex items-center gap-4">
            {/* Settings label */}
            <span className="text-xs font-medium text-gray-600 flex-shrink-0">⚙️ 설정</span>

            {/* Context Select */}
            <div className="flex items-center gap-1.5">
              <label className="text-[10px] text-gray-500">상황</label>
              <select
                value={lastContext}
                onChange={(e) => handleContextChange(e.target.value as PolishContext)}
                className="px-2 py-1 text-xs bg-white border border-gray-300 rounded-md focus:ring-2 focus:ring-purple-500 focus:border-purple-500"
              >
                {POLISH_CONTEXTS.map((ctx) => (
                  <option key={ctx.id} value={ctx.id}>
                    {ctx.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Channel Select */}
            <div className="flex items-center gap-1.5">
              <label className="text-[10px] text-gray-500">채널</label>
              <select
                value={lastChannel}
                onChange={(e) => handleChannelChange(e.target.value as PolishChannel)}
                className="px-2 py-1 text-xs bg-white border border-gray-300 rounded-md focus:ring-2 focus:ring-purple-500 focus:border-purple-500"
              >
                {POLISH_CHANNELS.map((ch) => (
                  <option key={ch.id} value={ch.id}>
                    {ch.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Model Select + Repolish button */}
            <div className="flex items-center gap-1.5">
              <label className="text-[10px] text-gray-500">모델</label>
              <select
                value={displayModel}
                onChange={(e) => handleModelChange(e.target.value as ModelProfileId)}
                className="px-2 py-1 text-xs bg-white border border-gray-300 rounded-md focus:ring-2 focus:ring-purple-500 focus:border-purple-500"
              >
                {modelProfiles.map((model) => (
                  <option key={model.id} value={model.id}>
                    {formatModelProfileOption(model, providerConfigs, defaultModel)}
                  </option>
                ))}
              </select>
              <button
                onClick={handleRepolish}
                disabled={isStreaming || !editableText.trim()}
                className="px-3 py-1 text-xs font-medium text-purple-700 bg-purple-50 border border-purple-300 rounded-md hover:bg-purple-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                다시 다듬기
              </button>
            </div>

            {/* Divider */}
            <div className="w-px h-5 bg-gray-300" />

            {/* Options */}
            <div className="flex items-center gap-1.5 flex-wrap">
              {POLISH_OPTIONS.map((opt) => (
                <label
                  key={opt.id}
                  className={`inline-flex items-center gap-1 px-2 py-1 text-[10px] rounded-full cursor-pointer transition-colors ${
                    lastOptions.includes(opt.id)
                      ? "bg-purple-100 text-purple-700 border border-purple-300"
                      : "bg-white text-gray-600 border border-gray-300 hover:bg-gray-50"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={lastOptions.includes(opt.id)}
                    onChange={() => handleOptionToggle(opt.id)}
                    className="sr-only"
                  />
                  <span>{opt.name}</span>
                </label>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Footer - 액션 버튼 */}
      <div className="flex flex-wrap items-center justify-end gap-2 px-4 py-3 border-t border-gray-200/50">
        <div className="hidden sm:flex items-center gap-1 text-[10px] text-gray-400 mr-auto">
          <span className="font-mono bg-gray-100 px-1.5 py-0.5 rounded">ESC</span>
          <span className="font-mono bg-gray-100 px-1.5 py-0.5 rounded">⌘↵</span>
        </div>
        <button
          onClick={handleTranslate}
          disabled={isStreaming || !hasResult}
          className="px-4 py-2 text-sm font-medium text-blue-700 bg-white border border-blue-300 rounded-lg hover:bg-blue-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          번역
        </button>
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
          className="px-4 py-2 text-sm font-medium text-white bg-purple-500 rounded-lg hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          title="바꾸기 (⌘+Enter)"
        >
          바꾸기
          <span className="ml-1.5 text-[10px] opacity-70">⌘↵</span>
        </button>
      </div>
    </div>
  );
}
