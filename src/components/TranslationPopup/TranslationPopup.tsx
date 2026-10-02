import { useEffect, useCallback, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslationStream } from "@/hooks/useTranslationStream";
import { useWindowDrag } from "@/hooks/useWindowDrag";
import { useClipboardStore, useSettingsStore } from "@/store";
import { Badge, Button, Icon, IconButton, Kbd, Select, Spinner } from "@/components/common";
import {
  DEFAULT_MODEL_PROFILE_ID,
  formatModelProfileOption,
  type ModelProfileId,
} from "@/types";

interface TranslationPopupProps {
  sourceText: string;
  onClose: () => void;
  onPreferredHeightChange?: (height: number) => void;
}

export function TranslationPopup({
  sourceText,
  onClose,
  onPreferredHeightChange,
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
        // Run hide + paste in one backend command to avoid a race where the
        // second invoke is dropped after hiding the WebView.
        const response = await invoke<{ success: boolean; error?: { code: string; message: string } }>(
          "hide_and_paste_text",
          { text: textToReplace }
        );
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

  // Check if source text has been modified
  const isSourceModified = editableText !== sourceText;

  const displayLanguage = detectedLanguage ?? "en";
  const targetLanguage = displayLanguage === "ko" ? "en" : "ko";
  const sourceLabel = displayLanguage === "ko" ? "한국어" : "English";
  const targetLabel = targetLanguage === "ko" ? "한국어" : "English";

  // Determine if we have content to display
  const hasResult = Boolean(fullText || streamedText);
  const resultStatusMessage = error
    ? "번역에 문제가 생겼습니다."
    : isStreaming
      ? "번역 중입니다."
      : hasResult
        ? "번역이 완료되었습니다."
        : "번역을 준비 중입니다.";

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

  useEffect(() => {
    schedulePreferredHeightUpdate();
  }, [
    editableText,
    fullText,
    streamedText,
    error,
    isStreaming,
    schedulePreferredHeightUpdate,
  ]);

  return (
    <div ref={containerRef} className="tone-accent animate-fade-in flex h-full w-full flex-col">
      <div className="grabber" aria-hidden="true" />

      {/* Toolbar - draggable area */}
      <header className="toolbar" onMouseDown={handleDragStart}>
        <IconButton icon="chevron-left" label="뒤로" variant="glass" iconSize={18} onClick={onClose} />

        <div className="flex items-center gap-2">
          <Icon name="translate" size={17} className="text-accent" />
          <h1 className="text-title text-label">번역</h1>
        </div>

        <div className="ml-auto flex items-center gap-2">
          {glossaryApplied && glossaryApplied.length > 0 && (
            <Badge icon="book">용어집 {glossaryApplied.length}개 적용</Badge>
          )}
          {fromCache && (
            <Badge tone="green" icon="check">
              캐시
            </Badge>
          )}
          <Kbd className="hidden sm:inline-flex" aria-hidden="true">
            ESC
          </Kbd>
          <IconButton icon="xmark" label="닫기 (ESC)" variant="glass" onClick={onClose} />
        </div>
      </header>

      {/* Source → result */}
      <div className="min-h-0 flex-1 px-4 pb-3 pt-3">
        <div className="flex h-full gap-3">
          {/* Source text (editable) */}
          <section className="flex min-w-0 flex-1 flex-col" aria-label="원문">
            <div className="mb-2 flex items-center gap-2 px-1">
              <h2 className="text-sub font-semibold text-label-2">원문</h2>
              <Badge>{sourceLabel}</Badge>
              {isSourceModified && <Badge tone="orange">수정됨</Badge>}
            </div>
            <div className="pane flex min-h-0 flex-1">
              <textarea
                ref={sourceTextareaRef}
                value={editableText}
                onChange={(e) => setEditableText(e.target.value)}
                className="h-full w-full resize-none rounded-pane bg-transparent px-4 py-3.5 text-reading text-label outline-none [overflow-wrap:anywhere] placeholder:text-label-3"
                placeholder="원문을 수정하여 다시 번역할 수 있습니다..."
              />
            </div>
          </section>

          {/* Direction */}
          <div className="flex items-center justify-center" aria-hidden="true">
            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-fill text-label-3">
              <Icon name="arrow-right" size={14} strokeWidth={2} />
            </span>
          </div>

          {/* Translated text */}
          <section className="flex min-w-0 flex-1 flex-col" aria-label="번역 결과 영역">
            <div className="mb-2 flex items-center gap-2 px-1">
              <h2 className="text-sub font-semibold text-label-2">번역 결과</h2>
              <Badge tone="accent">{targetLabel}</Badge>
            </div>
            <div
              ref={resultContainerRef}
              className={`pane pane-result ${error ? "tone-red" : ""} min-h-0 flex-1 overflow-y-auto px-4 py-3.5`}
              role="status"
              aria-live="polite"
              aria-atomic="true"
              aria-label="번역 결과"
            >
              <span className="sr-only">{resultStatusMessage}</span>
              {error ? (
                <div className="flex items-start gap-2.5">
                  <Icon name="warning" size={16} className="mt-0.5 shrink-0 text-red" />
                  <div className="min-w-0">
                    <p className="text-body font-medium text-red-fg">
                      번역에 문제가 생겼습니다. 다시 시도해 주세요.
                    </p>
                    <p className="selectable mt-1 break-words text-sub text-label-2">{error}</p>
                  </div>
                </div>
              ) : (fullText || streamedText) ? (
                <p className="selectable reading text-reading text-label">
                  {fullText || streamedText}
                  {isStreaming && (
                    <span className="animate-caret ml-0.5 inline-block h-[1.1em] w-[2px] translate-y-[3px] rounded-full bg-accent" />
                  )}
                </p>
              ) : (
                <div className="flex h-full min-h-[3rem] items-center justify-center gap-2 text-body text-label-3">
                  <Spinner size={16} />
                  <span>번역 중…</span>
                </div>
              )}
            </div>
          </section>
        </div>
      </div>

      {/* Actions */}
      <footer className="footbar flex shrink-0 flex-wrap items-center gap-2 px-4 py-3">
        <div className="mr-auto flex flex-wrap items-center gap-2">
          <label htmlFor="translate-model" className="text-sub text-label-3">
            모델
          </label>
          <Select
            id="translate-model"
            value={displayModel}
            onChange={(e) => handleModelChange(e.target.value as ModelProfileId)}
          >
            {modelProfiles.map((model) => (
              <option key={model.id} value={model.id}>
                {formatModelProfileOption(model, providerConfigs, defaultModel)}
              </option>
            ))}
          </Select>
          <Button
            variant="tinted"
            onClick={handleRetranslate}
            disabled={isStreaming || !editableText.trim()}
          >
            다시 번역
          </Button>
        </div>

        <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
          <Button
            variant={isSaved ? "tinted" : "secondary"}
            tone={isSaved ? "green" : "accent"}
            icon={isSaved ? "check" : undefined}
            onClick={handleSaveAsPostIt}
            disabled={isStreaming || !hasResult || isSaved}
          >
            {isSaved ? "저장됨!" : "메모로 저장"}
          </Button>
          <Button onClick={handleCopy} disabled={isStreaming || !hasResult}>
            복사
          </Button>
          <Button
            variant="primary"
            onClick={handleReplace}
            disabled={isStreaming || !hasResult}
            title="바꾸기 (⌘+Enter)"
          >
            바꾸기
            <span className="text-micro font-medium opacity-70" aria-hidden="true">
              ⌘↵
            </span>
          </Button>
        </div>
      </footer>
    </div>
  );
}
