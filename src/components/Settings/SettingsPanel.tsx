import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/store";
import { ApiKeyInput } from "./ApiKeyInput";
import type { ClaudeModel, PermissionStatus } from "@/types";

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
    <div className="h-full overflow-y-auto p-4">
      {/* 2열 그리드 레이아웃 */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* API 키 카드 - 노란색 */}
        <SettingsCard
          icon="🔑"
          title="API 키"
          color="yellow"
        >
          <ApiKeyInput hasApiKey={apiKeyStatus?.exists ?? false} />
        </SettingsCard>

        {/* 번역 설정 카드 - 파란색 */}
        <SettingsCard
          icon="🤖"
          title="번역 설정"
          color="blue"
        >
          {/* Model Selection */}
          <div className="space-y-2">
            <label className="block text-xs font-medium text-blue-700">
              Claude 모델
            </label>
            <select
              value={settings.preferredModel}
              onChange={(e) => handleModelChange(e.target.value as ClaudeModel)}
              className="w-full px-2 py-1.5 text-sm bg-white border border-blue-300 rounded-md focus:ring-2 focus:ring-blue-500"
              disabled={isLoading}
            >
              <option value="claude-haiku-4-5-20251001">Claude Haiku 4.5 (가장 빠름)</option>
              <option value="claude-sonnet-4-5-20250929">Claude Sonnet 4.5 (균형)</option>
              <option value="claude-opus-4-5-20251101">Claude Opus 4.5 (최고 품질)</option>
            </select>
          </div>

          {/* Auto Detect Language */}
          <div className="flex items-center justify-between mt-3">
            <label className="text-xs font-medium text-blue-700">
              언어 자동 감지
            </label>
            <ToggleSwitch
              enabled={settings.autoDetectLanguage}
              onChange={handleAutoDetectChange}
              disabled={isLoading}
              color="blue"
            />
          </div>
        </SettingsCard>

        {/* 단축키 카드 - 녹색 */}
        <SettingsCard
          icon="⌨️"
          title="단축키"
          color="green"
        >
          <div className="space-y-2">
            <label className="block text-xs font-medium text-green-700">
              더블 프레스 인터벌: {settings.doublePressInterval}ms
            </label>
            <input
              type="range"
              min="200"
              max="1000"
              step="50"
              value={settings.doublePressInterval}
              onChange={(e) => handleDoublePressIntervalChange(Number(e.target.value))}
              className="w-full accent-green-500"
              disabled={isLoading}
            />
            <p className="text-[10px] text-green-600">
              ⌘C⌘C 감지 시간 간격
            </p>
          </div>
        </SettingsCard>

        {/* 클립보드 카드 - 주황색 */}
        <SettingsCard
          icon="📋"
          title="클립보드"
          color="orange"
        >
          <div className="space-y-2">
            <label className="block text-xs font-medium text-orange-700">
              최대 저장 항목: {settings.maxHistoryCount}개
            </label>
            <input
              type="range"
              min="10"
              max="200"
              step="10"
              value={settings.maxHistoryCount}
              onChange={(e) => handleMaxHistoryChange(Number(e.target.value))}
              className="w-full accent-orange-500"
              disabled={isLoading}
            />
          </div>
        </SettingsCard>

        {/* 화면 표시 카드 - 핑크색 */}
        <SettingsCard
          icon="🎨"
          title="화면 표시"
          color="pink"
        >
          {/* Paste Delay */}
          <div className="space-y-2">
            <label className="block text-xs font-medium text-pink-700">
              붙여넣기 딜레이: {settings.pasteDelayMs}ms
            </label>
            <input
              type="range"
              min="50"
              max="500"
              step="25"
              value={settings.pasteDelayMs}
              onChange={(e) => handlePasteDelayChange(Number(e.target.value))}
              className="w-full accent-pink-500"
              disabled={isLoading}
            />
            <p className="text-[10px] text-pink-600">
              붙여넣기 실패 시 값을 높여보세요
            </p>
          </div>
        </SettingsCard>

        {/* 시스템 카드 - 회색 */}
        <SettingsCard
          icon="⚙️"
          title="시스템"
          color="gray"
        >
          {/* Launch at Login */}
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-gray-700">
              로그인 시 시작
            </label>
            <ToggleSwitch
              enabled={settings.launchAtLogin}
              onChange={handleLaunchAtLoginChange}
              disabled={isLoading}
              color="gray"
            />
          </div>

          {/* Accessibility Permission */}
          <div className="mt-3 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-gray-700">
                  접근성 권한
                </span>
                {accessibilityGranted === true && (
                  <span className="px-1.5 py-0.5 text-[10px] font-medium text-green-700 bg-green-100 rounded-full">
                    허용됨
                  </span>
                )}
                {accessibilityGranted === false && (
                  <span className="px-1.5 py-0.5 text-[10px] font-medium text-red-700 bg-red-100 rounded-full">
                    필요
                  </span>
                )}
              </div>
              {accessibilityGranted === false && (
                <button
                  onClick={handleRequestAccessibility}
                  className="px-2 py-1 text-[10px] font-medium text-white bg-blue-500 hover:bg-blue-600 rounded transition-colors"
                >
                  허용
                </button>
              )}
              {accessibilityGranted === true && (
                <button
                  onClick={checkAccessibilityStatus}
                  className="px-2 py-1 text-[10px] text-gray-600 hover:bg-gray-200 rounded transition-colors"
                >
                  새로고침
                </button>
              )}
            </div>
            <p className="text-[10px] text-gray-500">
              ⌘C⌘C 감지에 필요합니다
            </p>
          </div>
        </SettingsCard>
      </div>

      {/* 하단 정보 영역 */}
      <div className="mt-6 pt-4 border-t border-gray-200 flex items-center justify-between text-xs text-gray-500">
        <span>TransClip v0.1.6</span>
        <button
          type="button"
          className="flex items-center gap-1 text-gray-500 hover:text-blue-600 transition-colors"
          onClick={() => {
            import("@tauri-apps/plugin-shell").then(({ open }) => {
              open("https://github.com/dennis-hong/trans-clip/issues");
            });
          }}
        >
          <span>버그 제보 · 기능 제안</span>
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
          </svg>
        </button>
      </div>
    </div>
  );
}

// 설정 카드 컴포넌트
interface SettingsCardProps {
  icon: string;
  title: string;
  color: "yellow" | "blue" | "green" | "orange" | "pink" | "gray";
  children: React.ReactNode;
}

const colorClasses = {
  yellow: "bg-yellow-100 border-yellow-300",
  blue: "bg-blue-100 border-blue-300",
  green: "bg-green-100 border-green-300",
  orange: "bg-orange-100 border-orange-300",
  pink: "bg-pink-100 border-pink-300",
  gray: "bg-gray-100 border-gray-300",
};

const titleColors = {
  yellow: "text-yellow-800",
  blue: "text-blue-800",
  green: "text-green-800",
  orange: "text-orange-800",
  pink: "text-pink-800",
  gray: "text-gray-800",
};

function SettingsCard({ icon, title, color, children }: SettingsCardProps) {
  return (
    <div className={`p-4 rounded-lg border-2 shadow-md ${colorClasses[color]}`}>
      <div className={`flex items-center gap-2 mb-3 font-semibold text-sm ${titleColors[color]}`}>
        <span>{icon}</span>
        <span>{title}</span>
      </div>
      <div className="space-y-2">
        {children}
      </div>
    </div>
  );
}

// 토글 스위치 컴포넌트
interface ToggleSwitchProps {
  enabled: boolean;
  onChange: (enabled: boolean) => void;
  disabled?: boolean;
  color: "yellow" | "blue" | "green" | "orange" | "pink" | "gray";
}

const toggleColors = {
  yellow: "bg-yellow-500",
  blue: "bg-blue-500",
  green: "bg-green-500",
  orange: "bg-orange-500",
  pink: "bg-pink-500",
  gray: "bg-gray-500",
};

function ToggleSwitch({ enabled, onChange, disabled, color }: ToggleSwitchProps) {
  return (
    <button
      onClick={() => onChange(!enabled)}
      className={`relative w-9 h-5 rounded-full transition-colors ${
        enabled ? toggleColors[color] : "bg-gray-300"
      }`}
      disabled={disabled}
    >
      <span
        className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform ${
          enabled ? "translate-x-4" : ""
        }`}
      />
    </button>
  );
}
