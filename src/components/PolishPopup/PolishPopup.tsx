import { useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/common";
import { usePolish } from "@/hooks/usePolish";
import {
  usePolishStore,
  POLISH_CONTEXTS,
  POLISH_CHANNELS,
  POLISH_OPTIONS,
} from "@/store";
import type { PolishContext, PolishChannel, PolishOption } from "@/types";

interface PolishPopupProps {
  sourceText: string;
  onClose: () => void;
}

export function PolishPopup({ sourceText, onClose }: PolishPopupProps) {
  const popupRef = useRef<HTMLDivElement>(null);
  const { polish, isLoading, error, result } = usePolish();

  // Get last used settings from store
  const {
    lastContext,
    lastChannel,
    lastOptions,
    setLastContext,
    setLastChannel,
    toggleOption,
  } = usePolishStore();

  // Polish on mount and when settings change
  useEffect(() => {
    if (sourceText) {
      polish(sourceText, lastContext, lastChannel, lastOptions);
    }
  }, [sourceText, lastContext, lastChannel, lastOptions, polish]);

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

    const timeout = setTimeout(() => {
      document.addEventListener("mousedown", handleClickOutside);
    }, 100);

    return () => {
      clearTimeout(timeout);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [onClose]);

  const handleCopy = useCallback(async () => {
    if (result?.polishedText) {
      try {
        await invoke("set_clipboard", { text: result.polishedText });
        onClose();
      } catch (err) {
        console.error("Failed to copy:", err);
      }
    }
  }, [result?.polishedText, onClose]);

  const handleReplace = useCallback(async () => {
    if (result?.polishedText) {
      try {
        await invoke("hide_translation_popup");
        await new Promise((resolve) => setTimeout(resolve, 300));
        const response = await invoke<{
          success: boolean;
          error?: { code: string; message: string };
        }>("paste_text", { text: result.polishedText });
        console.log("paste_text response:", response);
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
  }, [result?.polishedText, onClose]);

  const handleRepolish = useCallback(() => {
    if (sourceText) {
      polish(sourceText, lastContext, lastChannel, lastOptions);
    }
  }, [sourceText, lastContext, lastChannel, lastOptions, polish]);

  const handleContextChange = (context: PolishContext) => {
    setLastContext(context);
  };

  const handleChannelChange = (channel: PolishChannel) => {
    setLastChannel(channel);
  };

  const handleOptionToggle = (option: PolishOption) => {
    toggleOption(option);
  };

  return (
    <div className="fixed inset-0 flex items-center justify-center p-4 bg-black/20">
      <div
        ref={popupRef}
        className="w-full max-w-4xl bg-white dark:bg-gray-900 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-200 flex items-center gap-2">
            <span>✏️</span>
            <span>글 다듬기</span>
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-200 dark:hover:text-gray-300 dark:hover:bg-gray-700 transition-colors"
          >
            <svg
              className="w-5 h-5"
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
        <div className="p-4 space-y-4">
          {/* Source Text */}
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400">
                📝 원문 (러프한 초안)
              </span>
            </div>
            <div className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 max-h-32 overflow-y-auto">
              <p className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
                {sourceText}
              </p>
            </div>
          </div>

          {/* Settings */}
          <div className="space-y-3">
            <div className="flex items-center gap-2 text-xs font-medium text-gray-500 dark:text-gray-400">
              ⚙️ 설정
            </div>

            {/* Context & Channel Dropdowns */}
            <div className="flex gap-3">
              {/* Context Select */}
              <div className="flex-1">
                <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                  상황
                </label>
                <select
                  value={lastContext}
                  onChange={(e) =>
                    handleContextChange(e.target.value as PolishContext)
                  }
                  className="w-full px-3 py-2 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:text-gray-200"
                >
                  {POLISH_CONTEXTS.map((ctx) => (
                    <option key={ctx.id} value={ctx.id}>
                      {ctx.name}
                    </option>
                  ))}
                </select>
              </div>

              {/* Channel Select */}
              <div className="flex-1">
                <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                  채널
                </label>
                <select
                  value={lastChannel}
                  onChange={(e) =>
                    handleChannelChange(e.target.value as PolishChannel)
                  }
                  className="w-full px-3 py-2 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:text-gray-200"
                >
                  {POLISH_CHANNELS.map((ch) => (
                    <option key={ch.id} value={ch.id}>
                      {ch.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {/* Options */}
            <div className="flex flex-wrap gap-2">
              {POLISH_OPTIONS.map((opt) => (
                <label
                  key={opt.id}
                  className={`inline-flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-full cursor-pointer transition-colors ${
                    lastOptions.includes(opt.id)
                      ? "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 border border-blue-300 dark:border-blue-700"
                      : "bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700 hover:bg-gray-200 dark:hover:bg-gray-700"
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

          {/* Polished Result */}
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400">
                ✨ 정돈된 결과
              </span>
            </div>
            <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 min-h-[100px] max-h-48 overflow-y-auto">
              {isLoading ? (
                <div className="flex items-center justify-center h-20">
                  <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                    <svg
                      className="animate-spin h-4 w-4"
                      fill="none"
                      viewBox="0 0 24 24"
                    >
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
              ) : error ? (
                <div className="text-sm text-red-600 dark:text-red-400">
                  {error}
                </div>
              ) : result?.polishedText ? (
                <p className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
                  {result.polishedText}
                </p>
              ) : null}
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 bg-gray-50 dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700">
          <Button
            variant="secondary"
            size="sm"
            onClick={handleRepolish}
            disabled={isLoading}
          >
            다시 다듬기
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={handleCopy}
            disabled={isLoading || !result?.polishedText}
          >
            복사
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleReplace}
            disabled={isLoading || !result?.polishedText}
          >
            바꾸기
          </Button>
        </div>
      </div>
    </div>
  );
}
