import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  ClipboardItem,
  ClipboardHistoryResponse,
  DeleteResponse,
  PinResponse,
  ClearHistoryResponse,
} from "@/types";

let latestFetchRequestId = 0;

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
  clearAll: () => Promise<boolean>;
  togglePin: (id: string) => Promise<boolean>;
  addItem: (item: ClipboardItem) => void;
  createItem: (content: string) => Promise<ClipboardItem | null>;
  updateItemContent: (id: string, content: string) => Promise<boolean>;
  clearError: () => void;
}

export const useClipboardStore = create<ClipboardStore>((set) => ({
  items: [],
  total: 0,
  hasMore: false,
  isLoading: false,
  error: null,

  fetchHistory: async (options = {}) => {
    const requestId = ++latestFetchRequestId;
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

      if (requestId !== latestFetchRequestId) {
        return;
      }

      if (options.offset && options.offset > 0) {
        // Append to existing items for pagination
        set((state) => {
          if (requestId !== latestFetchRequestId) {
            return state;
          }

          return {
            items: [...state.items, ...response.items],
            total: response.total,
            hasMore: response.hasMore,
            isLoading: false,
          };
        });
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
      if (requestId !== latestFetchRequestId) {
        return;
      }
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

  clearAll: async () => {
    try {
      const response = await invoke<ClearHistoryResponse>("clear_clipboard_history");

      if (response.success) {
        set({
          items: [],
          total: 0,
          hasMore: false,
        });
        return true;
      } else {
        set({ error: response.error?.message ?? "Failed to clear history" });
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

  createItem: async (content: string) => {
    try {
      const response = await invoke<ClipboardItem>("create_clipboard_item", {
        content,
      });

      // Add to the top of the list
      set((state) => ({
        items: [response, ...state.items],
        total: state.total + 1,
      }));

      return response;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  },

  updateItemContent: async (id: string, content: string) => {
    try {
      const response = await invoke<ClipboardItem>("update_clipboard_item", {
        id,
        content,
      });

      // Update the item in the list
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? response : item
        ),
      }));

      return true;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  },

  clearError: () => set({ error: null }),
}));
