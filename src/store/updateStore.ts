import { create } from "zustand";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";

interface UpdateStore {
  currentVersion: string | null;
  latestVersion: string | null;
  releaseNotes: string | null;
  hasUpdate: boolean;
  isChecking: boolean;
  isDownloading: boolean;
  progress: number;
  dismissedVersion: string | null;
  error: string | null;

  initCurrentVersion: () => Promise<void>;
  checkForUpdate: (force?: boolean) => Promise<boolean>;
  installUpdate: () => Promise<boolean>;
  dismissUpdate: () => void;
  clearError: () => void;
}

let pendingUpdate: Update | null = null;

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function closePendingUpdate() {
  if (!pendingUpdate) {
    return;
  }

  const previous = pendingUpdate;
  pendingUpdate = null;
  void previous.close().catch(() => {
    // Ignore close errors; this is only a cleanup path.
  });
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  currentVersion: null,
  latestVersion: null,
  releaseNotes: null,
  hasUpdate: false,
  isChecking: false,
  isDownloading: false,
  progress: 0,
  dismissedVersion: null,
  error: null,

  initCurrentVersion: async () => {
    if (get().currentVersion) {
      return;
    }

    try {
      const version = await getVersion();
      set({ currentVersion: version });
    } catch (error) {
      set({ error: getErrorMessage(error) });
    }
  },

  checkForUpdate: async (force = false) => {
    set({ isChecking: true, error: null });

    try {
      const currentVersion = await getVersion();
      const update = await check();

      if (!update) {
        closePendingUpdate();
        set({
          currentVersion,
          latestVersion: null,
          releaseNotes: null,
          hasUpdate: false,
          isChecking: false,
          isDownloading: false,
          progress: 0,
        });
        return false;
      }

      closePendingUpdate();
      pendingUpdate = update;

      const dismissedVersion = get().dismissedVersion;
      const shouldShowUpdate = force || dismissedVersion !== update.version;

      set({
        currentVersion,
        latestVersion: update.version,
        releaseNotes: update.body ?? null,
        hasUpdate: shouldShowUpdate,
        isChecking: false,
        progress: 0,
      });

      return shouldShowUpdate;
    } catch (error) {
      closePendingUpdate();
      set({
        isChecking: false,
        isDownloading: false,
        progress: 0,
        hasUpdate: false,
        error: force ? getErrorMessage(error) : null,
      });
      return false;
    }
  },

  installUpdate: async () => {
    if (!pendingUpdate) {
      set({ error: "설치 가능한 업데이트가 없습니다." });
      return false;
    }

    set({ isDownloading: true, progress: 0, error: null });

    try {
      let downloadedBytes = 0;
      let totalBytes = 0;
      const installingVersion = pendingUpdate.version;

      await pendingUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
          downloadedBytes = 0;
          set({ progress: 0 });
          return;
        }

        if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            const progress = Math.min(100, Math.round((downloadedBytes / totalBytes) * 100));
            set({ progress });
          }
          return;
        }

        set({ progress: 100 });
      });

      closePendingUpdate();
      set({
        latestVersion: installingVersion,
        hasUpdate: false,
        isDownloading: false,
        progress: 100,
        dismissedVersion: null,
      });

      return true;
    } catch (error) {
      set({
        isDownloading: false,
        error: getErrorMessage(error),
      });
      return false;
    }
  },

  dismissUpdate: () => {
    set((state) => ({
      hasUpdate: false,
      dismissedVersion: state.latestVersion,
      error: null,
    }));
  },

  clearError: () => set({ error: null }),
}));
