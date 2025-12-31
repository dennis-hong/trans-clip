import { useState, useCallback } from "react";
import { Button } from "@/components/common";
import type { GlossaryEntry } from "@/types";

interface GlossaryEditorProps {
  entry?: GlossaryEntry;
  onSave: (data: {
    keyword: string;
    description: string;
  }) => Promise<void>;
  onCancel: () => void;
  isLoading?: boolean;
}

export function GlossaryEditor({
  entry,
  onSave,
  onCancel,
  isLoading,
}: GlossaryEditorProps) {
  const [keyword, setKeyword] = useState(entry?.keyword ?? "");
  const [description, setDescription] = useState(entry?.description ?? "");
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      setError(null);

      if (!keyword.trim()) {
        setError("Keyword is required");
        return;
      }

      if (!description.trim()) {
        setError("Description is required");
        return;
      }

      try {
        await onSave({
          keyword: keyword.trim(),
          description: description.trim(),
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to save entry");
      }
    },
    [keyword, description, onSave]
  );

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
        {entry ? "Edit Entry" : "Add New Entry"}
      </h3>

      {error && (
        <div className="p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
        </div>
      )}

      {/* Keyword */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          Keyword
        </label>
        <input
          type="text"
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          placeholder="e.g., RFC, Lunit, API"
          maxLength={100}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
          disabled={isLoading}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500">
          The term that should be handled specially during translation
        </p>
      </div>

      {/* Description */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          Description
        </label>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Describe how this keyword should be translated or handled. e.g., 'Company name, keep as Lunit in English or 루닛 in Korean'"
          rows={4}
          maxLength={500}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none resize-none"
          disabled={isLoading}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500">
          Provide context for the AI to use when translating this term
        </p>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-2 pt-2">
        <Button variant="ghost" size="sm" type="button" onClick={onCancel}>
          Cancel
        </Button>
        <Button variant="primary" size="sm" type="submit" loading={isLoading}>
          {entry ? "Update" : "Add"}
        </Button>
      </div>
    </form>
  );
}
