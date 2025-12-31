import { useEffect, useState, useCallback } from "react";
import { useGlossaryStore } from "@/store";
import { Button, Modal } from "@/components/common";
import { GlossaryEditor } from "./GlossaryEditor";
import type { GlossaryEntry } from "@/types";

export function GlossaryList() {
  const [searchQuery, setSearchQuery] = useState("");
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<GlossaryEntry | undefined>();

  const {
    entries,
    isLoading,
    error,
    fetchEntries,
    addEntry,
    updateEntry,
    deleteEntry,
  } = useGlossaryStore();

  useEffect(() => {
    fetchEntries({
      searchQuery: searchQuery || undefined,
    });
  }, [fetchEntries, searchQuery]);

  const handleAddNew = useCallback(() => {
    setEditingEntry(undefined);
    setIsEditorOpen(true);
  }, []);

  const handleEdit = useCallback((entry: GlossaryEntry) => {
    setEditingEntry(entry);
    setIsEditorOpen(true);
  }, []);

  const handleSave = useCallback(
    async (data: {
      keyword: string;
      description: string;
    }) => {
      if (editingEntry) {
        await updateEntry(editingEntry.id, {
          keyword: data.keyword,
          description: data.description,
        });
      } else {
        await addEntry(data);
      }
      setIsEditorOpen(false);
      setEditingEntry(undefined);
    },
    [editingEntry, addEntry, updateEntry]
  );

  const handleDelete = useCallback(
    async (id: string) => {
      if (confirm("Are you sure you want to delete this entry?")) {
        await deleteEntry(id);
      }
    },
    [deleteEntry]
  );

  const handleCancel = useCallback(() => {
    setIsEditorOpen(false);
    setEditingEntry(undefined);
  }, []);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-3 border-b border-gray-200 dark:border-gray-700 space-y-3">
        {/* Search */}
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
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search keywords..."
            className="w-full pl-10 pr-4 py-2 bg-gray-100 dark:bg-gray-800 rounded-lg border-none text-sm focus:ring-2 focus:ring-blue-500 outline-none"
          />
        </div>

        {/* Add Button */}
        <Button variant="primary" size="sm" onClick={handleAddNew} className="w-full">
          Add New Entry
        </Button>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {isLoading && entries.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
          </div>
        ) : error ? (
          <div className="text-center py-8">
            <p className="text-sm text-red-500">{error}</p>
          </div>
        ) : entries.length === 0 ? (
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
                d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
              />
            </svg>
            <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
              {searchQuery ? "No entries match your search" : "No glossary entries yet"}
            </p>
            <p className="text-xs text-gray-400 dark:text-gray-500">
              Add custom terms for better translations
            </p>
          </div>
        ) : (
          entries.map((entry) => (
            <div
              key={entry.id}
              className="p-3 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-semibold text-blue-600 dark:text-blue-400">
                      {entry.keyword}
                    </span>
                  </div>
                  <p className="mt-1 text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
                    {entry.description}
                  </p>
                  <p className="mt-2 text-xs text-gray-400 dark:text-gray-500">
                    Used {entry.usageCount} times
                  </p>
                </div>
                <div className="flex-shrink-0 flex items-center gap-1">
                  <button
                    onClick={() => handleEdit(entry)}
                    className="p-1 text-gray-400 hover:text-blue-500 rounded transition-colors"
                    title="Edit"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                      />
                    </svg>
                  </button>
                  <button
                    onClick={() => handleDelete(entry.id)}
                    className="p-1 text-gray-400 hover:text-red-500 rounded transition-colors"
                    title="Delete"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                      />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Editor Modal */}
      <Modal isOpen={isEditorOpen} onClose={handleCancel}>
        <GlossaryEditor
          entry={editingEntry}
          onSave={handleSave}
          onCancel={handleCancel}
          isLoading={isLoading}
        />
      </Modal>
    </div>
  );
}
