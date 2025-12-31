import { useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/common";
import { SourceText } from "./SourceText";
import { TranslatedText } from "./TranslatedText";
import { useTranslation } from "@/hooks/useTranslation";

interface TranslationPopupProps {
  sourceText: string;
  onClose: () => void;
}

export function TranslationPopup({
  sourceText,
  onClose,
}: TranslationPopupProps) {
  const popupRef = useRef<HTMLDivElement>(null);
  const { translate, isLoading, error, result } = useTranslation();

  // Translate on mount
  useEffect(() => {
    if (sourceText) {
      translate(sourceText);
    }
  }, [sourceText, translate]);

  // Handle ESC key to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Handle click outside to close
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popupRef.current && !popupRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    // Delay adding listener to prevent immediate close
    const timeout = setTimeout(() => {
      document.addEventListener("mousedown", handleClickOutside);
    }, 100);

    return () => {
      clearTimeout(timeout);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [onClose]);

  const handleCopy = useCallback(async () => {
    if (result?.translatedText) {
      try {
        await invoke("set_clipboard", { text: result.translatedText });
        onClose();
      } catch (err) {
        console.error("Failed to copy:", err);
      }
    }
  }, [result?.translatedText, onClose]);

  const handleReplace = useCallback(async () => {
    if (result?.translatedText) {
      try {
        // First hide the popup to return focus to the original app
        await invoke("hide_translation_popup");
        // Give the original app time to regain focus (increased delay)
        await new Promise(resolve => setTimeout(resolve, 300));
        // Now simulate paste in the original app
        const response = await invoke<{ success: boolean; error?: { code: string; message: string } }>("paste_text", { text: result.translatedText });
        console.log("paste_text response:", response);
        if (!response.success && response.error) {
          console.error("Paste failed:", response.error.code, response.error.message);
        }
        onClose();
      } catch (err) {
        console.error("Failed to replace:", err);
      }
    }
  }, [result?.translatedText, onClose]);

  const detectedLanguage = result?.detectedLanguage ?? "en";
  const targetLanguage = detectedLanguage === "ko" ? "en" : "ko";

  return (
    <div className="fixed inset-0 flex items-center justify-center p-4 bg-black/20">
      <div
        ref={popupRef}
        className="w-full max-w-md bg-white dark:bg-gray-900 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-200">
            TransClip
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-200 dark:hover:text-gray-300 dark:hover:bg-gray-700 transition-colors"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
        <div className="p-4 space-y-4">
          <SourceText text={sourceText} language={detectedLanguage} />

          <div className="flex justify-center">
            <svg
              className="w-5 h-5 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 14l-7 7m0 0l-7-7m7 7V3"
              />
            </svg>
          </div>

          <TranslatedText
            text={result?.translatedText ?? ""}
            language={targetLanguage}
            isLoading={isLoading}
            error={error ?? undefined}
          />

          {/* Glossary info */}
          {result?.glossaryApplied && result.glossaryApplied.length > 0 && (
            <p className="text-xs text-gray-500 dark:text-gray-400">
              {result.glossaryApplied.length} glossary term(s) applied
            </p>
          )}

          {/* Cache indicator */}
          {result?.fromCache && (
            <p className="text-xs text-gray-500 dark:text-gray-400">
              From cache
            </p>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 bg-gray-50 dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700">
          <Button
            variant="secondary"
            size="sm"
            onClick={handleCopy}
            disabled={isLoading || !result?.translatedText}
          >
            Copy
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleReplace}
            disabled={isLoading || !result?.translatedText}
          >
            Replace
          </Button>
        </div>
      </div>
    </div>
  );
}
