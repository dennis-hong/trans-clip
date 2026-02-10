import { invoke } from "@tauri-apps/api/core";

const DEFAULT_TIMEOUT_MS = 30_000;

/**
 * Wraps Tauri invoke with a timeout to prevent infinite hangs.
 */
export async function invokeWithTimeout<T>(
  cmd: string,
  args?: Record<string, unknown>,
  timeoutMs: number = DEFAULT_TIMEOUT_MS
): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(
        () => reject(new Error(`invoke("${cmd}") timed out after ${timeoutMs}ms`)),
        timeoutMs
      )
    ),
  ]);
}
