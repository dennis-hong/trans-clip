import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
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
                <div className="text-center space-y-3">
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
