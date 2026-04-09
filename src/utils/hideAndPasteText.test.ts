import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hideAndPasteText } from "./hideAndPasteText";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const showMock = vi.fn();
const setFocusMock = vi.fn();

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);
const getCurrentWindowMock = vi.mocked(getCurrentWindow);

describe("hideAndPasteText", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    showMock.mockReset();
    setFocusMock.mockReset();
    showMock.mockResolvedValue(undefined);
    setFocusMock.mockResolvedValue(undefined);
    getCurrentWindowMock.mockReturnValue({
      show: showMock,
      setFocus: setFocusMock,
    } as unknown as ReturnType<typeof getCurrentWindow>);
  });

  it("returns successful paste results without restoring the window", async () => {
    invokeMock.mockResolvedValue({ success: true });

    await expect(hideAndPasteText("translated")).resolves.toEqual({ success: true });
    expect(showMock).not.toHaveBeenCalled();
    expect(setFocusMock).not.toHaveBeenCalled();
  });

  it("restores the popup window when the backend paste reports failure", async () => {
    invokeMock.mockResolvedValue({
      success: false,
      error: { code: "PASTE_FAILED", message: "boom" },
    });

    await expect(hideAndPasteText("translated")).resolves.toEqual({
      success: false,
      error: { code: "PASTE_FAILED", message: "boom" },
    });

    expect(showMock).toHaveBeenCalledTimes(1);
    expect(setFocusMock).toHaveBeenCalledTimes(1);
  });

  it("restores the popup window before rethrowing invoke failures", async () => {
    invokeMock.mockRejectedValue(new Error("invoke failed"));

    await expect(hideAndPasteText("translated")).rejects.toThrow("invoke failed");
    expect(showMock).toHaveBeenCalledTimes(1);
    expect(setFocusMock).toHaveBeenCalledTimes(1);
  });
});
