import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  ClipboardItem,
  ClipboardHistoryResponse,
  DeleteResponse,
  PinResponse,
} from "@/types";

interface ClipboardStore {
  items: ClipboardItem[];
  total: number;
  hasMore: boolean;
  isLoading: boolean;
  error: string | null;

  // Actions
  fetchHistory: (options?: {
    limit?: number;
    offset?: number;
    searchQuery?: string;
  }) => Promise<void>;
  deleteItem: (id: string) => Promise<boolean>;
  togglePin: (id: string) => Promise<boolean>;
  addItem: (item: ClipboardItem) => void;
  clearError: () => void;
}

export const useClipboardStore = create<ClipboardStore>((set) => ({
  items: [],
  total: 0,
  hasMore: false,
  isLoading: false,
  error: null,

  fetchHistory: async (options = {}) => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<ClipboardHistoryResponse>(
        "get_clipboard_history",
        {
          limit: options.limit ?? 50,
          offset: options.offset ?? 0,
          searchQuery: options.searchQuery,
        }
      );

      if (options.offset && options.offset > 0) {
        // Append to existing items for pagination
        set((state) => ({
          items: [...state.items, ...response.items],
          total: response.total,
          hasMore: response.hasMore,
          isLoading: false,
        }));
      } else {
        // Replace items for fresh fetch
        set({
          items: response.items,
          total: response.total,
          hasMore: response.hasMore,
          isLoading: false,
        });
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
    }
  },

  deleteItem: async (id: string) => {
    try {
      const response = await invoke<DeleteResponse>("delete_clipboard_item", {
        id,
      });

      if (response.success) {
        set((state) => ({
          items: state.items.filter((item) => item.id !== id),
          total: state.total - 1,
        }));
        return true;
      } else {
        set({ error: response.error?.message ?? "Failed to delete item" });
        return false;
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  },

  togglePin: async (id: string) => {
    try {
      const response = await invoke<PinResponse>("toggle_pin_clipboard_item", {
        id,
      });

      if (response.success) {
        set((state) => ({
          items: state.items.map((item) =>
            item.id === id ? { ...item, isPinned: response.isPinned } : item
          ),
        }));
        return true;
      } else {
        set({ error: response.error?.message ?? "Failed to toggle pin" });
        return false;
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  },

  addItem: (item: ClipboardItem) => {
    set((state) => {
      // Check if item with same content already exists
      const existingIndex = state.items.findIndex(
        (i) => i.content === item.content
      );

      if (existingIndex !== -1) {
        // Update existing item's timestamp
        const newItems = [...state.items];
        newItems.splice(existingIndex, 1);
        return { items: [item, ...newItems] };
      }

      return { items: [item, ...state.items], total: state.total + 1 };
    });
  },

  clearError: () => set({ error: null }),
}));
