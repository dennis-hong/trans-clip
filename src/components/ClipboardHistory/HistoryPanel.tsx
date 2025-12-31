import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/store";
import { HistoryItem } from "./HistoryItem";
import { Toast } from "@/components/common";
import type { ClipboardItem, ClipboardChangedPayload } from "@/types";

export function HistoryPanel() {
  const [searchQuery, setSearchQuery] = useState("");
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const { items, isLoading, error, hasMore, fetchHistory, deleteItem, togglePin } =
    useClipboardStore();

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  // Listen for clipboard changes
  useEffect(() => {
    const unlisten = listen<ClipboardChangedPayload>("clipboard_changed", () => {
      // Refresh history when clipboard changes
      fetchHistory();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchHistory]);

  const handleSearch = useCallback(
    (query: string) => {
      setSearchQuery(query);
      fetchHistory({ searchQuery: query || undefined });
    },
    [fetchHistory]
  );

  const handleCopy = useCallback(async (item: ClipboardItem) => {
    try {
      await invoke("set_clipboard", { text: item.content });
      setToast({ message: "Copied to clipboard!", type: "success" });
    } catch (err) {
      console.error("Failed to copy:", err);
      setToast({ message: "Failed to copy", type: "error" });
    }
  }, []);

  const handleDelete = useCallback(
    async (id: string) => {
      await deleteItem(id);
    },
    [deleteItem]
  );

  const handleTogglePin = useCallback(
    async (id: string) => {
      await togglePin(id);
    },
    [togglePin]
  );

  const handleLoadMore = useCallback(() => {
    fetchHistory({ offset: items.length, searchQuery: searchQuery || undefined });
  }, [fetchHistory, items.length, searchQuery]);

  return (
    <div className="flex flex-col h-full">
      {/* Toast notification */}
      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={() => setToast(null)}
        />
      )}

      {/* Search */}
      <div className="p-3 border-b border-gray-200 dark:border-gray-700">
        <div className="relative">
          <svg
            className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder="Search clipboard history..."
            className="w-full pl-10 pr-4 py-2 bg-gray-100 dark:bg-gray-800 rounded-lg border-none text-sm focus:ring-2 focus:ring-blue-500 outline-none"
          />
        </div>
      </div>

      {/* History List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {isLoading && items.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
          </div>
        ) : error ? (
          <div className="text-center py-8">
            <p className="text-sm text-red-500">{error}</p>
          </div>
        ) : items.length === 0 ? (
          <div className="text-center py-8">
            <svg
              className="mx-auto w-12 h-12 text-gray-300 dark:text-gray-600"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
              />
            </svg>
            <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
              {searchQuery ? "No items match your search" : "No clipboard history yet"}
            </p>
            <p className="text-xs text-gray-400 dark:text-gray-500">
              Copy some text to get started
            </p>
          </div>
        ) : (
          <>
            {items.map((item) => (
              <HistoryItem
                key={item.id}
                item={item}
                onCopy={handleCopy}
                onDelete={handleDelete}
                onTogglePin={handleTogglePin}
              />
            ))}

            {hasMore && (
              <button
                onClick={handleLoadMore}
                disabled={isLoading}
                className="w-full py-2 text-sm text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors"
              >
                {isLoading ? "Loading..." : "Load more"}
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
