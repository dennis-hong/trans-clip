import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { invokeWithTimeout } from "./invokeWithTimeout";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("invokeWithTimeout", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns the invoke result when it completes in time", async () => {
    invokeMock.mockResolvedValue("ok");

    await expect(invokeWithTimeout<string>("ping", { id: 1 }, 1000)).resolves.toBe("ok");
    expect(invokeMock).toHaveBeenCalledWith("ping", { id: 1 });
  });

  it("rejects when invoke exceeds timeout", async () => {
    vi.useFakeTimers();
    invokeMock.mockImplementation(() => new Promise(() => {}));

    const pending = invokeWithTimeout("slow_cmd", undefined, 25);
    const assertion = expect(pending).rejects.toThrow(
      'invoke("slow_cmd") timed out after 25ms'
    );
    await vi.advanceTimersByTimeAsync(26);
    await assertion;
  });
});
