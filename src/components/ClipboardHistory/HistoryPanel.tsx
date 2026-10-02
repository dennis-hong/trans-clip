import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/store";
import { HistoryItem } from "./HistoryItem";
import { Button, Icon, IconButton, Modal, Spinner, Toast } from "@/components/common";
import type { ClipboardItem, ClipboardChangedPayload } from "@/types";

export function HistoryPanel() {
  const [searchQuery, setSearchQuery] = useState("");
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const { items, isLoading, error, hasMore, fetchHistory, deleteItem, togglePin, clearAll } =
    useClipboardStore();

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  // Listen for clipboard changes (debounced to avoid redundant fetches)
  const clipboardDebounceRef = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => {
    const unlisten = listen<ClipboardChangedPayload>("clipboard_changed", () => {
      clearTimeout(clipboardDebounceRef.current);
      clipboardDebounceRef.current = setTimeout(() => fetchHistory(), 100);
    });

    return () => {
      clearTimeout(clipboardDebounceRef.current);
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

  const handleClearAll = useCallback(async () => {
    const success = await clearAll();
    if (success) {
      setToast({ message: "History cleared!", type: "success" });
    } else {
      setToast({ message: "Failed to clear history", type: "error" });
    }
    setShowClearConfirm(false);
  }, [clearAll]);

  return (
    <div className="flex h-full flex-col">
      {/* Toast notification */}
      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={() => setToast(null)}
        />
      )}

      {/* Clear Confirmation Modal */}
      <Modal
        isOpen={showClearConfirm}
        onClose={() => setShowClearConfirm(false)}
        title="Clear All History?"
        showCloseButton={false}
      >
        <p className="mb-4 text-body text-label-2">
          This will permanently delete all {items.length} clipboard items. This action cannot be undone.
        </p>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setShowClearConfirm(false)}>Cancel</Button>
          <Button variant="primary" tone="red" onClick={handleClearAll}>
            Clear All
          </Button>
        </div>
      </Modal>

      {/* Search */}
      <div className="p-3 shadow-[0_0.5px_0_var(--separator)]">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Icon
              name="search"
              size={14}
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-label-3"
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              placeholder="Search clipboard history..."
              aria-label="Search clipboard history"
              className="field field-search"
            />
          </div>
          {items.length > 0 && (
            <IconButton
              icon="trash"
              label="Clear all history"
              danger
              onClick={() => setShowClearConfirm(true)}
            />
          )}
        </div>
      </div>

      {/* History List */}
      <div className="flex-1 space-y-2 overflow-y-auto p-3">
        {isLoading && items.length === 0 ? (
          <div className="flex items-center justify-center py-8 text-label-3">
            <Spinner size={20} />
          </div>
        ) : error ? (
          <div className="py-8 text-center">
            <p className="text-body text-red-fg">{error}</p>
          </div>
        ) : items.length === 0 ? (
          <div className="py-8 text-center">
            <Icon name="clipboard" size={32} strokeWidth={1.3} className="mx-auto text-label-3" />
            <p className="mt-2 text-body font-medium text-label-2">
              {searchQuery ? "No items match your search" : "No clipboard history yet"}
            </p>
            <p className="text-sub text-label-3">Copy some text to get started</p>
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
              <Button
                variant="tinted"
                className="w-full"
                onClick={handleLoadMore}
                disabled={isLoading}
              >
                {isLoading ? "Loading..." : "Load more"}
              </Button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
