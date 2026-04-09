import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

type TauriListenEvent = {
  payload: Record<string, unknown>;
};

type ListenerCallback = (event: TauriListenEvent) => void;

const listeners = new Map<string, ListenerCallback>();
const invokeMock = vi.fn();
const listenMock = vi.fn((eventName: string, callback: ListenerCallback) => {
  listeners.set(eventName, callback);
  return Promise.resolve(() => {
    listeners.delete(eventName);
  });
});
const hideMock = vi.fn();
const onResizedMock = vi.fn(() => Promise.resolve(() => {}));
const scaleFactorMock = vi.fn(() => Promise.resolve(1));
const relaunchMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    hide: hideMock,
    onResized: onResizedMock,
    scaleFactor: scaleFactorMock,
  }),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: (...args: unknown[]) => relaunchMock(...args),
}));

vi.mock("@/components/DrawerPanel", () => ({
  DrawerPanel: ({ onTranslate, onPolish, onClose }: { onTranslate?: (text: string) => void; onPolish?: (text: string) => void; onClose?: () => void }) => (
    <div data-testid="drawer-panel">
      <button data-testid="drawer-translate" onClick={() => onTranslate?.("from-history")}>translate</button>
      <button data-testid="drawer-polish" onClick={() => onPolish?.("from-history")}>polish</button>
      <button data-testid="drawer-close" onClick={() => onClose?.()}>close</button>
    </div>
  ),
}));

vi.mock("@/components/TranslationPopup", () => ({
  TranslationPopup: ({ sourceText, onClose }: { sourceText: string; onClose: () => void }) => (
    <div data-testid="translation-popup">
      <span>{sourceText}</span>
      <button data-testid="translation-close" onClick={onClose}>close</button>
    </div>
  ),
}));

vi.mock("@/components/PolishPopup", () => ({
  PolishPopup: ({
    sourceText,
    onClose,
    onTranslate,
  }: {
    sourceText: string;
    onClose: () => void;
    onTranslate?: (text: string) => void;
  }) => (
    <div data-testid="polish-popup">
      <span>{sourceText}</span>
      <button data-testid="polish-translate" onClick={() => onTranslate?.("polished-text")}>translate</button>
      <button data-testid="polish-close" onClick={onClose}>close</button>
    </div>
  ),
}));

function emit(eventName: string, payload: Record<string, unknown>) {
  const callback = listeners.get(eventName);
  if (!callback) {
    throw new Error(`No listener registered for ${eventName}`);
  }
  callback({ payload });
}

describe("App", () => {
  beforeEach(() => {
    listeners.clear();
    invokeMock.mockReset();
    listenMock.mockClear();
    hideMock.mockReset();
    onResizedMock.mockReset();
    onResizedMock.mockImplementation(() => Promise.resolve(() => {}));
    scaleFactorMock.mockReset();
    scaleFactorMock.mockResolvedValue(1);
    relaunchMock.mockReset();

    invokeMock.mockImplementation((command: string) => {
      if (command === "check_accessibility_permission") {
        return Promise.resolve({ granted: true });
      }
      if (command === "move_to_monitor" || command === "set_drawer_mode") {
        return Promise.resolve(null);
      }
      return Promise.resolve(null);
    });
  });

  it("shows translation popup from double copy and hides window on close", async () => {
    await act(async () => {
      render(<App />);
    });

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });

    act(() => {
      emit("double_copy_detected", { text: "hello" });
    });

    expect(await screen.findByTestId("translation-popup")).toBeInTheDocument();
    expect(screen.getByText("hello")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("translation-close"));

    await waitFor(() => {
      expect(hideMock).toHaveBeenCalledTimes(1);
    });
  });

  it("returns to history when closing popup opened from history", async () => {
    await act(async () => {
      render(<App />);
    });

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });

    act(() => {
      emit("show_history", {});
    });
    expect(await screen.findByTestId("drawer-panel")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("drawer-translate"));
    expect(await screen.findByTestId("translation-popup")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("translation-close"));

    await waitFor(() => {
      expect(screen.getByTestId("drawer-panel")).toBeInTheDocument();
    });
    expect(hideMock).not.toHaveBeenCalled();
  });

  it("returns to history after translating from a polish popup opened from history", async () => {
    await act(async () => {
      render(<App />);
    });

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });

    act(() => {
      emit("show_history", {});
    });
    expect(await screen.findByTestId("drawer-panel")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("drawer-polish"));
    expect(await screen.findByTestId("polish-popup")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("polish-translate"));
    expect(await screen.findByTestId("translation-popup")).toBeInTheDocument();
    expect(screen.getByText("polished-text")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("translation-close"));

    await waitFor(() => {
      expect(screen.getByTestId("drawer-panel")).toBeInTheDocument();
    });
    expect(hideMock).not.toHaveBeenCalled();
  });
});
