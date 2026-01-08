import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { DrawerPanel } from "@/components/DrawerPanel";
import { TranslationPopup } from "@/components/TranslationPopup";
import { PolishPopup } from "@/components/PolishPopup";
import type { DoubleCopyPayload, PolishPayload } from "@/types";

type PopupMode = "none" | "translate" | "polish";

function App() {
  const [popupMode, setPopupMode] = useState<PopupMode>("none");
  const [sourceText, setSourceText] = useState<string | null>(null);
  const [hasAccessibility, setHasAccessibility] = useState<boolean | null>(null);

  // Check accessibility permission on mount
  useEffect(() => {
    if (hasAccessibility === true) return;

    const checkPermission = async () => {
      try {
        const result = await invoke<{ granted: boolean }>("check_accessibility_permission");
        setHasAccessibility(result.granted);
      } catch (err) {
        console.error("Failed to check accessibility permission:", err);
        setHasAccessibility(false);
      }
    };

    checkPermission();
    const interval = setInterval(checkPermission, 3000);
    return () => clearInterval(interval);
  }, [hasAccessibility]);

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

  // Note: Keyboard shortcuts for monitor switching are handled in DrawerPanel.tsx
  // to avoid duplicate calls and to update the currentMonitor state

  // Listen for double copy events (Cmd+CC for translation)
  useEffect(() => {
    const unlisten = listen<DoubleCopyPayload>("double_copy_detected", (event) => {
      const { text } = event.payload;
      if (text && text.trim()) {
        setSourceText(text);
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
        setPopupMode("polish");
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Adjust window size when popup mode changes
  useEffect(() => {
    const updateWindowMode = async () => {
      if (popupMode === "translate" || popupMode === "polish") {
        try {
          await invoke("set_drawer_mode", { mode: "popup" });
        } catch (err) {
          console.error("Failed to set popup mode:", err);
        }
      }
    };
    updateWindowMode();
  }, [popupMode]);

  const handleClosePopup = useCallback(async () => {
    setPopupMode("none");
    setSourceText(null);
    // Return to expanded drawer mode
    try {
      await invoke("set_drawer_mode", { mode: "expanded" });
    } catch (err) {
      console.error("Failed to set drawer mode:", err);
    }
  }, []);

  // Show translation/polish popup if active
  if (popupMode === "translate" && sourceText) {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <div className="h-full flex flex-col bg-white/95 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
          <TranslationPopup sourceText={sourceText} onClose={handleClosePopup} />
        </div>
      </div>
    );
  }

  if (popupMode === "polish" && sourceText) {
    return (
      <div className="h-screen w-full overflow-hidden bg-transparent">
        <div className="h-full flex flex-col bg-white/95 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
          <PolishPopup sourceText={sourceText} onClose={handleClosePopup} />
        </div>
      </div>
    );
  }

  // Default: show drawer panel
  return (
    <div className="h-screen w-full overflow-hidden bg-transparent">
      <DrawerPanel hasAccessibility={hasAccessibility} />
    </div>
  );
}

export default App;
