import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { DrawerPanel } from "@/components/DrawerPanel";
import { TranslationPopup } from "@/components/TranslationPopup";
import { PolishPopup } from "@/components/PolishPopup";
import { useUpdateStore } from "@/store";
import type { DoubleCopyPayload, PolishPayload, ShowHistoryPayload } from "@/types";

type PopupMode = "none" | "translate" | "polish" | "history";

function App() {
  const [popupMode, setPopupMode] = useState<PopupMode>("none");
  const [sourceText, setSourceText] = useState<string | null>(null);
  const [hasAccessibility, setHasAccessibility] = useState<boolean | null>(null);
  const [openedFromHistory, setOpenedFromHistory] = useState(false);
  const [showRestartDialog, setShowRestartDialog] = useState(false);
  const [openSettingsSignal, setOpenSettingsSignal] = useState(0);
  const prevAccessibilityRef = useRef<boolean | null>(null);
  const checkForUpdate = useUpdateStore((state) => state.checkForUpdate);

  // Check accessibility permission on mount and detect changes
  // Uses exponential backoff: 2s → 4s → 8s → ... → 30s max, stops after 3 minutes
  useEffect(() => {
    if (hasAccessibility === true && !showRestartDialog) return;

    let timeoutId: ReturnType<typeof setTimeout>;
    let elapsedMs = 0;
    const MAX_POLL_MS = 3 * 60 * 1000; // Stop polling after 3 minutes
    const MAX_INTERVAL_MS = 30_000;
    let currentInterval = 2000;

    const checkPermission = async () => {
      try {
        const result = await invoke<{ granted: boolean }>("check_accessibility_permission");

        // Detect permission change from false to true
        if (prevAccessibilityRef.current === false && result.granted === true) {
          setShowRestartDialog(true);
        }

        prevAccessibilityRef.current = result.granted;
        setHasAccessibility(result.granted);

        // Stop polling once granted
        if (result.granted) return;
      } catch (err) {
        console.error("Failed to check accessibility permission:", err);
        setHasAccessibility(false);
        prevAccessibilityRef.current = false;
      }

      elapsedMs += currentInterval;
      if (elapsedMs < MAX_POLL_MS) {
        currentInterval = Math.min(currentInterval * 2, MAX_INTERVAL_MS);
        timeoutId = setTimeout(checkPermission, currentInterval);
      }
    };

    checkPermission();
    return () => clearTimeout(timeoutId);
  }, [hasAccessibility, showRestartDialog]);

  // Handle app restart
  const handleRestart = useCallback(async () => {
    try {
      await relaunch();
    } catch (err) {
      console.error("Failed to restart app:", err);
    }
  }, []);

  const openSettingsFromMenu = useCallback(() => {
    setSourceText(null);
    setOpenedFromHistory(false);
    setPopupMode("history");
    setOpenSettingsSignal((prev) => prev + 1);
  }, []);

  // Position window at bottom on mount
  useEffect(() => {
    const positionWindow = async () => {
      try {
        await invoke("move_to_monitor", { monitorIndex: 0, anchor: "bottom" });
      } catch (err) {
        console.error("Failed to position window:", err);
      }
    };
    positionWindow();
  }, []);

  // Check for updates once at startup (non-blocking)
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      void checkForUpdate().catch((err) => {
        console.error("Failed to check for updates:", err);
      });
    }, 3000);

    return () => clearTimeout(timeoutId);
  }, [checkForUpdate]);

  // Note: Keyboard shortcuts for monitor switching are handled in DrawerPanel.tsx
  // to avoid duplicate calls and to update the currentMonitor state

  // Listen for double copy events (Cmd+CC for translation)
  useEffect(() => {
    const unlisten = listen<DoubleCopyPayload>("double_copy_detected", (event) => {
      const { text } = event.payload;
      if (text && text.trim()) {
        setSourceText(text);
        setOpenedFromHistory(false);
        setPopupMode("translate");
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for polish events (Cmd+DD for polishing)
  useEffect(() => {
    const unlisten = listen<PolishPayload>("polish_detected", (event) => {
      const { text } = event.payload;
      if (text && text.trim()) {
        setSourceText(text);
        setOpenedFromHistory(false);
        setPopupMode("polish");
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for show_history events (Cmd+Shift+V or tray click)
  useEffect(() => {
    const unlisten = listen<ShowHistoryPayload>("show_history", () => {
      setPopupMode("history");
      setSourceText(null);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for open_settings events from menu
  useEffect(() => {
    const unlisten = listen("open_settings", () => {
      openSettingsFromMenu();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [openSettingsFromMenu]);

  // Listen for manual update check events from menu
  useEffect(() => {
    const unlisten = listen("check_for_updates", () => {
      openSettingsFromMenu();
      void checkForUpdate(true).catch((err) => {
        console.error("Failed to check updates from menu:", err);
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [checkForUpdate, openSettingsFromMenu]);

  // Adjust window size when popup mode changes
  useEffect(() => {
    const updateWindowMode = async () => {
      if (popupMode === "translate" || popupMode === "polish") {
        try {
          await invoke("set_drawer_mode", { mode: "popup" });
        } catch (err) {
          console.error("Failed to set popup mode:", err);
        }
      } else if (popupMode === "history") {
        try {
          await invoke("set_drawer_mode", { mode: "expanded" });
        } catch (err) {
          console.error("Failed to set expanded mode:", err);
        }
      }
    };
    updateWindowMode();
  }, [popupMode]);

  // Hide window completely (stealth mode)
  const hideWindow = useCallback(async () => {
    try {
      const window = getCurrentWindow();
      await window.hide();
    } catch (err) {
      console.error("Failed to hide window:", err);
    }
  }, []);

  const handleClosePopup = useCallback(async () => {
    setSourceText(null);
    
    // If opened from history, go back to history instead of hiding
    if (openedFromHistory) {
      setOpenedFromHistory(false);
      setPopupMode("history");
    } else {
      setPopupMode("none");
      // Hide window instead of staying visible (stealth mode)
      await hideWindow();
    }
  }, [hideWindow, openedFromHistory]);

  // Handle closing history panel
  const handleCloseHistory = useCallback(async () => {
    setPopupMode("none");
    await hideWindow();
  }, [hideWindow]);

  // Handle translate from history
  const handleTranslateFromHistory = useCallback((text: string) => {
    setSourceText(text);
    setOpenedFromHistory(true);
    setPopupMode("translate");
  }, []);

  // Handle polish from history
  const handlePolishFromHistory = useCallback((text: string) => {
    setSourceText(text);
    setOpenedFromHistory(true);
    setPopupMode("polish");
  }, []);

  // Handle translate from polish popup
  const handleTranslateFromPolish = useCallback((text: string) => {
    setSourceText(text);
    setOpenedFromHistory(false);
    setPopupMode("translate");
  }, []);

  // Restart dialog for accessibility permission
  if (showRestartDialog) {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <div className="h-full flex flex-col items-center justify-center bg-gradient-to-b from-gray-50/98 to-white/98 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
          <div className="bg-white rounded-2xl shadow-xl p-6 max-w-sm mx-4 border border-gray-100">
            <div className="flex items-center justify-center w-12 h-12 mx-auto mb-4 bg-green-100 rounded-full">
              <svg className="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <h2 className="text-lg font-semibold text-gray-900 text-center mb-2">
              접근성 권한이 허용되었습니다
            </h2>
            <p className="text-sm text-gray-600 text-center mb-6">
              기능을 활성화하려면 앱을 재시작해야 합니다.
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => setShowRestartDialog(false)}
                className="flex-1 px-4 py-2.5 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
              >
                나중에
              </button>
              <button
                onClick={handleRestart}
                className="flex-1 px-4 py-2.5 text-sm font-medium text-white bg-blue-500 hover:bg-blue-600 rounded-lg transition-colors"
              >
                지금 재시작
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Show translation/polish popup if active
  if (popupMode === "translate" && sourceText) {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <div className="h-full flex flex-col bg-gradient-to-b from-gray-50/95 to-white/95 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
          <TranslationPopup sourceText={sourceText} onClose={handleClosePopup} />
        </div>
      </div>
    );
  }

  if (popupMode === "polish" && sourceText) {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <div className="h-full flex flex-col bg-gradient-to-b from-gray-50/95 to-white/95 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
          <PolishPopup sourceText={sourceText} onClose={handleClosePopup} onTranslate={handleTranslateFromPolish} />
        </div>
      </div>
    );
  }

  // History mode: show drawer panel with close handler
  if (popupMode === "history") {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <DrawerPanel 
          hasAccessibility={hasAccessibility} 
          onClose={handleCloseHistory}
          isStealthMode={true}
          onTranslate={handleTranslateFromHistory}
          onPolish={handlePolishFromHistory}
          openSettingsSignal={openSettingsSignal}
        />
      </div>
    );
  }

  // Default (none): should not render as window is hidden
  // But if somehow visible, show drawer panel
  return (
    <div className="h-screen w-full overflow-hidden bg-transparent">
      <DrawerPanel 
        hasAccessibility={hasAccessibility} 
        onClose={handleCloseHistory}
        isStealthMode={true}
        onTranslate={handleTranslateFromHistory}
        onPolish={handlePolishFromHistory}
        openSettingsSignal={openSettingsSignal}
      />
    </div>
  );
}

export default App;
