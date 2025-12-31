import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  GlossaryEntry,
  GlossaryListResponse,
  DeleteResponse,
  ImportGlossaryResponse,
  ExportGlossaryResponse,
} from "@/types";

interface GlossaryStore {
  entries: GlossaryEntry[];
  total: number;
  isLoading: boolean;
  error: string | null;

  // Actions
  fetchEntries: (options?: {
    searchQuery?: string;
    sortBy?: "keyword" | "usageCount" | "createdAt";
    sortOrder?: "asc" | "desc";
  }) => Promise<void>;
  addEntry: (entry: {
    keyword: string;
    description: string;
  }) => Promise<GlossaryEntry | null>;
  updateEntry: (
    id: string,
    updates: { keyword?: string; description?: string }
  ) => Promise<boolean>;
  deleteEntry: (id: string) => Promise<boolean>;
  importGlossary: (
    filePath: string,
    format: "csv" | "json",
    overwrite: boolean
  ) => Promise<ImportGlossaryResponse>;
  exportGlossary: (
    filePath: string,
    format: "csv" | "json"
  ) => Promise<ExportGlossaryResponse>;
  clearError: () => void;
}

export const useGlossaryStore = create<GlossaryStore>((set) => ({
  entries: [],
  total: 0,
  isLoading: false,
  error: null,

  fetchEntries: async (options = {}) => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<GlossaryListResponse>(
        "get_glossary_entries",
        {
          searchQuery: options.searchQuery,
          sortBy: options.sortBy ?? "keyword",
          sortOrder: options.sortOrder ?? "asc",
        }
      );

      set({
        entries: response.entries,
        total: response.total,
        isLoading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
    }
  },

  addEntry: async (entry) => {
    set({ isLoading: true, error: null });
    try {
      const newEntry = await invoke<GlossaryEntry>("add_glossary_entry", entry);

      set((state) => ({
        entries: [...state.entries, newEntry],
        total: state.total + 1,
        isLoading: false,
      }));

      return newEntry;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
      return null;
    }
  },

  updateEntry: async (id, updates) => {
    set({ isLoading: true, error: null });
    try {
      const updatedEntry = await invoke<GlossaryEntry>(
        "update_glossary_entry",
        {
          id,
          keyword: updates.keyword,
          description: updates.description,
        }
      );

      set((state) => ({
        entries: state.entries.map((e) => (e.id === id ? updatedEntry : e)),
        isLoading: false,
      }));

      return true;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        isLoading: false,
      });
      return false;
    }
  },

  deleteEntry: async (id) => {
    try {
      const response = await invoke<DeleteResponse>("delete_glossary_entry", {
        id,
      });

      if (response.success) {
        set((state) => ({
          entries: state.entries.filter((e) => e.id !== id),
          total: state.total - 1,
        }));
        return true;
      } else {
        set({ error: response.error?.message ?? "Failed to delete entry" });
        return false;
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  },

  importGlossary: async (filePath, format, overwrite) => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<ImportGlossaryResponse>("import_glossary", {
        filePath,
        format,
        overwrite,
      });

      set({ isLoading: false });
      return response;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({
        error: message,
        isLoading: false,
      });
      return { imported: 0, skipped: 0, errors: [{ line: 0, message }] };
    }
  },

  exportGlossary: async (filePath, format) => {
    set({ isLoading: true, error: null });
    try {
      const response = await invoke<ExportGlossaryResponse>("export_glossary", {
        filePath,
        format,
      });

      set({ isLoading: false });
      return response;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({
        error: message,
        isLoading: false,
      });
      return { success: false, exportedCount: 0 };
    }
  },

  clearError: () => set({ error: null }),
}));
