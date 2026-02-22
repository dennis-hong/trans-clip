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
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  const timeoutPromise = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(
      () => reject(new Error(`invoke("${cmd}") timed out after ${timeoutMs}ms`)),
      timeoutMs
    );
  });

  try {
    return await Promise.race([invoke<T>(cmd, args), timeoutPromise]);
  } finally {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
    }
  }
}
