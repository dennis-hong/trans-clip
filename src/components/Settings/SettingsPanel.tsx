import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/store";
import { ApiKeyInput } from "./ApiKeyInput";
import type { ClaudeModel, PopupPosition, PermissionStatus } from "@/types";

export function SettingsPanel() {
  const {
    settings,
    apiKeyStatus,
    fetchSettings,
    updateSettings,
    fetchApiKeyStatus,
    isLoading,
  } = useSettingsStore();

  const [accessibilityGranted, setAccessibilityGranted] = useState<boolean | null>(null);

  const checkAccessibilityStatus = async () => {
    try {
      const status = await invoke<PermissionStatus>("check_accessibility_permission");
      setAccessibilityGranted(status.granted);
    } catch (err) {
      console.error("Failed to check accessibility:", err);
    }
  };

  useEffect(() => {
    fetchSettings();
    fetchApiKeyStatus();
    checkAccessibilityStatus();
  }, [fetchSettings, fetchApiKeyStatus]);

  const handleModelChange = (model: ClaudeModel) => {
    updateSettings({ preferredModel: model });
  };

  const handlePopupPositionChange = (position: PopupPosition) => {
    updateSettings({ popupPosition: position });
  };

  const handleMaxHistoryChange = (value: number) => {
    updateSettings({ maxHistoryCount: value });
  };

  const handleDoublePressIntervalChange = (value: number) => {
    updateSettings({ doublePressInterval: value });
  };

  const handleAutoDetectChange = (enabled: boolean) => {
    updateSettings({ autoDetectLanguage: enabled });
  };

  const handleLaunchAtLoginChange = (enabled: boolean) => {
    updateSettings({ launchAtLogin: enabled });
  };

  const handlePasteDelayChange = (value: number) => {
    updateSettings({ pasteDelayMs: value });
  };

  const handleRequestAccessibility = async () => {
    try {
      await invoke("request_accessibility_permission");
      // Re-check after a short delay
      setTimeout(checkAccessibilityStatus, 1000);
    } catch (err) {
      console.error("Failed to request accessibility:", err);
    }
  };

  if (!settings) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
      </div>
    );
  }

  return (
    <div className="p-4 space-y-6 overflow-y-auto h-full">
      {/* API Key Section */}
      <section>
        <ApiKeyInput hasApiKey={apiKeyStatus?.exists ?? false} />
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* Translation Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          Translation
        </h3>

        {/* Model Selection */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Claude Model
          </label>
          <select
            value={settings.preferredModel}
            onChange={(e) => handleModelChange(e.target.value as ClaudeModel)}
            className="w-full px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 text-sm"
            disabled={isLoading}
          >
            <option value="claude-haiku-4-5-20251001">Claude Haiku 4.5 (Fastest)</option>
            <option value="claude-sonnet-4-5-20250929">Claude Sonnet 4.5 (Balanced)</option>
            <option value="claude-opus-4-5-20251101">Claude Opus 4.5 (Best)</option>
          </select>
        </div>

        {/* Auto Detect Language */}
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Auto-detect language
          </label>
          <button
            onClick={() => handleAutoDetectChange(!settings.autoDetectLanguage)}
            className={`relative w-11 h-6 rounded-full transition-colors ${
              settings.autoDetectLanguage ? "bg-blue-500" : "bg-gray-300 dark:bg-gray-600"
            }`}
            disabled={isLoading}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
                settings.autoDetectLanguage ? "translate-x-5" : ""
              }`}
            />
          </button>
        </div>
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* Shortcut Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          Keyboard Shortcut
        </h3>

        {/* Double Press Interval */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Double-press interval: {settings.doublePressInterval}ms
          </label>
          <input
            type="range"
            min="200"
            max="1000"
            step="50"
            value={settings.doublePressInterval}
            onChange={(e) => handleDoublePressIntervalChange(Number(e.target.value))}
            className="w-full"
            disabled={isLoading}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Time window for Cmd+C+C detection
          </p>
        </div>
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* UI Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          Appearance
        </h3>

        {/* Popup Position */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Popup Position
          </label>
          <select
            value={settings.popupPosition}
            onChange={(e) => handlePopupPositionChange(e.target.value as PopupPosition)}
            className="w-full px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 text-sm"
            disabled={isLoading}
          >
            <option value="cursor">At cursor</option>
            <option value="center">Screen center</option>
            <option value="top-right">Top right</option>
          </select>
        </div>
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* Clipboard Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          Clipboard History
        </h3>

        {/* Max History Count */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Maximum items: {settings.maxHistoryCount}
          </label>
          <input
            type="range"
            min="10"
            max="200"
            step="10"
            value={settings.maxHistoryCount}
            onChange={(e) => handleMaxHistoryChange(Number(e.target.value))}
            className="w-full"
            disabled={isLoading}
          />
        </div>
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* Replace & Paste Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          Replace & Paste
        </h3>

        {/* Paste Delay */}
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Paste delay: {settings.pasteDelayMs}ms
          </label>
          <input
            type="range"
            min="50"
            max="500"
            step="25"
            value={settings.pasteDelayMs}
            onChange={(e) => handlePasteDelayChange(Number(e.target.value))}
            className="w-full"
            disabled={isLoading}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Delay between switching to previous app and pasting. Increase if paste fails.
          </p>
        </div>
      </section>

      <hr className="border-gray-200 dark:border-gray-700" />

      {/* System Settings */}
      <section className="space-y-4">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          System
        </h3>

        {/* Launch at Login */}
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Launch at login
          </label>
          <button
            onClick={() => handleLaunchAtLoginChange(!settings.launchAtLogin)}
            className={`relative w-11 h-6 rounded-full transition-colors ${
              settings.launchAtLogin ? "bg-blue-500" : "bg-gray-300 dark:bg-gray-600"
            }`}
            disabled={isLoading}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
                settings.launchAtLogin ? "translate-x-5" : ""
              }`}
            />
          </button>
        </div>

        {/* Accessibility Permission */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Accessibility Permission
              </span>
              {accessibilityGranted === true && (
                <span className="px-2 py-0.5 text-xs font-medium text-green-700 bg-green-100 dark:text-green-300 dark:bg-green-900/30 rounded-full">
                  Granted
                </span>
              )}
              {accessibilityGranted === false && (
                <span className="px-2 py-0.5 text-xs font-medium text-red-700 bg-red-100 dark:text-red-300 dark:bg-red-900/30 rounded-full">
                  Not Granted
                </span>
              )}
            </div>
            {accessibilityGranted === false && (
              <button
                onClick={handleRequestAccessibility}
                className="px-3 py-1 text-xs font-medium text-white bg-blue-500 hover:bg-blue-600 rounded-lg transition-colors"
              >
                Grant
              </button>
            )}
            {accessibilityGranted === true && (
              <button
                onClick={checkAccessibilityStatus}
                className="px-3 py-1 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
              >
                Refresh
              </button>
            )}
          </div>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Required for Cmd+C+C detection. Grant permission in System Settings &gt; Privacy &amp; Security &gt; Accessibility
          </p>
        </div>
      </section>
    </div>
  );
}
