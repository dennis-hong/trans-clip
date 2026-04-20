import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

const invokeMock = vi.fn();

const settingsStoreState = {
  settings: {
    maxHistoryCount: 50,
    preferredModel: "claude-sonnet-4-6",
    autoDetectLanguage: true,
    doublePressInterval: 500,
    translationCacheDays: 7,
    showSourceApp: true,
    popupPosition: "cursor",
    launchAtLogin: false,
    pasteDelayMs: 150,
  },
  apiKeyStatus: { exists: false },
  fetchSettings: vi.fn(),
  updateSettings: vi.fn(),
  fetchApiKeyStatus: vi.fn(),
  setApiKey: vi.fn(),
  deleteApiKey: vi.fn(),
  isLoading: false,
  error: null,
};

const updateStoreState = {
  currentVersion: "0.1.19",
  latestVersion: null,
  hasUpdate: false,
  isChecking: false,
  isDownloading: false,
  progress: 0,
  error: null,
  initCurrentVersion: vi.fn(),
  checkForUpdate: vi.fn(),
  installUpdate: vi.fn(),
  dismissUpdate: vi.fn(),
  clearError: vi.fn(),
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn(),
}));

vi.mock("@/store", () => ({
  useSettingsStore: () => settingsStoreState,
  useUpdateStore: () => updateStoreState,
}));

vi.mock("./ApiKeyInput", () => ({
  ApiKeyInput: () => <div data-testid="api-key-input" />,
}));

describe("SettingsPanel", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "check_accessibility_permission") {
        return Promise.resolve({ granted: true });
      }

      return Promise.resolve(null);
    });

    settingsStoreState.fetchSettings.mockClear();
    settingsStoreState.updateSettings.mockClear();
    settingsStoreState.fetchApiKeyStatus.mockClear();
    settingsStoreState.setApiKey.mockClear();
    settingsStoreState.deleteApiKey.mockClear();
    updateStoreState.initCurrentVersion.mockClear();
    updateStoreState.checkForUpdate.mockClear();
    updateStoreState.installUpdate.mockClear();
    updateStoreState.dismissUpdate.mockClear();
    updateStoreState.clearError.mockClear();
  });

  it("shows the updated shortcut map and accessibility helper copy", async () => {
    render(<SettingsPanel />);

    expect(screen.getByText("연타 감지 간격: 500ms")).toBeInTheDocument();
    expect(screen.getByText("⌥⌥, ⌘C⌘C, ⌘E⌘E 감지 시간")).toBeInTheDocument();
    expect(screen.getByText("현재 단축키")).toBeInTheDocument();
    expect(screen.getByText("창 열기 / 숨기기")).toBeInTheDocument();
    expect(screen.getByText("선택 텍스트 번역")).toBeInTheDocument();
    expect(screen.getByText("선택 텍스트 다듬기")).toBeInTheDocument();
    expect(screen.getByText("전역 단축키 감지에 필요합니다")).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("허용됨")).toBeInTheDocument();
    });
  });

  it("updates the shared double-press interval setting from the shortcut slider", async () => {
    render(<SettingsPanel />);

    await waitFor(() => {
      expect(screen.getByText("허용됨")).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText("연타 감지 간격"), {
      target: { value: "350" },
    });

    expect(settingsStoreState.updateSettings).toHaveBeenCalledWith({
      doublePressInterval: 350,
    });
  });
});
