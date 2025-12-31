import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { TranslationPopup } from "@/components/TranslationPopup";
import { SettingsPanel } from "@/components/Settings/SettingsPanel";
import { HistoryPanel } from "@/components/ClipboardHistory/HistoryPanel";
import { GlossaryList } from "@/components/GlossaryManager/GlossaryList";
import type { DoubleCopyPayload } from "@/types";

type Tab = "translate" | "history" | "glossary" | "settings";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("translate");
  const [sourceText, setSourceText] = useState<string | null>(null);
  const [isTranslating, setIsTranslating] = useState(false);
  const [hasAccessibility, setHasAccessibility] = useState<boolean | null>(null);

  // Check accessibility permission on mount
  useEffect(() => {
    // Skip polling if permission is already granted
    if (hasAccessibility === true) {
      return;
    }

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
    
    // Re-check periodically only if permission is not yet granted
    const interval = setInterval(checkPermission, 3000);
    return () => clearInterval(interval);
  }, [hasAccessibility]);

  // Listen for double copy events from backend
  useEffect(() => {
    const unlisten = listen<DoubleCopyPayload>(
      "double_copy_detected",
      (event) => {
        const { text } = event.payload;
        if (text && text.trim()) {
          setSourceText(text);
          setIsTranslating(true);
          setActiveTab("translate");
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleClosePopup = useCallback(async () => {
    setIsTranslating(false);
    setSourceText(null);
  }, []);

  // Manual translate trigger for testing
  const handleManualTranslate = async () => {
    try {
      // Get clipboard text
      const text = await navigator.clipboard.readText();
      if (text && text.trim()) {
        setSourceText(text);
        setIsTranslating(true);
      }
    } catch (err) {
      console.error("Failed to read clipboard:", err);
    }
  };

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: "translate", label: "Translate", icon: "🌐" },
    { id: "history", label: "History", icon: "📋" },
    { id: "glossary", label: "Glossary", icon: "📖" },
    { id: "settings", label: "Settings", icon: "⚙️" },
  ];

  return (
    <div className="flex flex-col h-screen bg-white dark:bg-gray-900">
      {/* Tab Navigation */}
      <div className="flex border-b border-gray-200 dark:border-gray-700">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex-1 flex items-center justify-center gap-1 px-2 py-2 text-sm font-medium whitespace-nowrap transition-colors ${
              activeTab === tab.id
                ? "text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400"
                : "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
            }`}
          >
            <span>{tab.icon}</span>
            <span>{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === "translate" && (
          <div className="h-full flex flex-col">
            {isTranslating && sourceText ? (
              <TranslationPopup sourceText={sourceText} onClose={handleClosePopup} />
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center p-4">
                <div className="text-center space-y-4">
                  {/* Accessibility Permission Warning */}
                  {hasAccessibility === false && (
                    <div className="mb-4 p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg max-w-sm">
                      <div className="flex items-start gap-3">
                        <svg
                          className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                          />
                        </svg>
                        <div className="text-left">
                          <p className="text-sm font-medium text-amber-800 dark:text-amber-200">
                            Accessibility Permission Required
                          </p>
                          <p className="mt-1 text-xs text-amber-700 dark:text-amber-300">
                            Required for <span className="font-mono font-medium">Cmd+C+C</span> detection.
                          </p>
                          <p className="mt-2 text-xs text-amber-600 dark:text-amber-400">
                            Grant permission in:<br />
                            <span className="font-medium">System Settings → Privacy & Security → Accessibility</span>
                          </p>
                          <button
                            onClick={() => invoke("open_accessibility_settings")}
                            className="mt-3 px-3 py-1.5 bg-amber-500 hover:bg-amber-600 text-white text-xs font-medium rounded-md transition-colors"
                          >
                            Open Settings
                          </button>
                        </div>
                      </div>
                    </div>
                  )}

                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    <span className="font-medium text-gray-700 dark:text-gray-300">Cmd+C+C</span> to translate clipboard
                  </p>
                  <button
                    onClick={handleManualTranslate}
                    className="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium transition-colors"
                  >
                    Translate Now
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === "history" && <HistoryPanel />}

        {activeTab === "glossary" && <GlossaryList />}

        {activeTab === "settings" && <SettingsPanel />}
      </div>
    </div>
  );
}

export default App;
