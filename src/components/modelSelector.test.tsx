import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PolishPopup } from "./PolishPopup";
import { TranslationPopup } from "./TranslationPopup";
import { usePolishStore, useSettingsStore } from "@/store";
import type { ClaudeModel, UserSettings } from "@/types";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  polish: vi.fn(),
  translate: vi.fn(),
  handleDragStart: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
}));

vi.mock("@/hooks/useWindowDrag", () => ({
  useWindowDrag: () => ({
    handleDragStart: mocks.handleDragStart,
  }),
}));

vi.mock("@/hooks/useTranslationStream", () => ({
  useTranslationStream: () => ({
    translate: mocks.translate,
    isStreaming: false,
    streamedText: "번역됨",
    fullText: "번역됨",
    detectedLanguage: "en",
    fromCache: false,
    glossaryApplied: [],
    error: null,
  }),
}));

vi.mock("@/hooks/usePolishStream", () => ({
  usePolishStream: () => ({
    polish: mocks.polish,
    isStreaming: false,
    streamedText: "다듬어짐",
    fullText: "다듬어짐",
    error: null,
  }),
}));

function makeSettings(preferredModel: ClaudeModel): UserSettings {
  return {
    maxHistoryCount: 50,
    preferredModel,
    autoDetectLanguage: true,
    doublePressInterval: 500,
    translationCacheDays: 30,
    showSourceApp: true,
    popupPosition: "cursor",
    launchAtLogin: false,
    pasteDelayMs: 120,
    anthropicBaseUrl: "https://api.anthropic.com",
  };
}

describe("popup model selectors", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.polish.mockReset();
    mocks.translate.mockReset();
    mocks.handleDragStart.mockReset();

    useSettingsStore.setState({
      settings: makeSettings("claude-haiku-4-5-20251001"),
      apiKeyStatus: null,
      isLoading: false,
      error: null,
    });

    usePolishStore.setState({
      lastContext: "peer-discussion",
      lastChannel: "slack-message",
      lastOptions: [],
    });
  });

  it("shows the configured translation default as a real model option", () => {
    render(<TranslationPopup sourceText="hello" onClose={vi.fn()} />);

    const modelSelect = screen.getByDisplayValue("Haiku 4.5 (기본, 가장 빠름)");
    expect(screen.queryByRole("option", { name: "기본값" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "다시 번역" }));
    expect(mocks.translate).toHaveBeenLastCalledWith("hello", undefined);

    fireEvent.change(modelSelect, { target: { value: "claude-opus-4-7" } });
    fireEvent.click(screen.getByRole("button", { name: "다시 번역" }));
    expect(mocks.translate).toHaveBeenLastCalledWith("hello", "claude-opus-4-7");
  });

  it("shows the configured polish default as a real model option", () => {
    render(<PolishPopup sourceText="hello" onClose={vi.fn()} />);

    const modelSelect = screen.getAllByRole("combobox")[2];
    expect(modelSelect).toHaveDisplayValue("Haiku 4.5 (기본, 가장 빠름)");
    expect(screen.queryByRole("option", { name: "기본값" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "다시 다듬기" }));
    expect(mocks.polish).toHaveBeenLastCalledWith(
      "hello",
      "peer-discussion",
      "slack-message",
      [],
      undefined
    );
  });
});
