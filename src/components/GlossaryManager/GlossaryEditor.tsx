import { useState, useCallback } from "react";
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
        setError("용어를 입력해주세요");
        return;
      }

      if (!description.trim()) {
        setError("설명을 입력해주세요");
        return;
      }

      try {
        await onSave({
          keyword: keyword.trim(),
          description: description.trim(),
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : "저장에 실패했습니다");
      }
    },
    [keyword, description, onSave]
  );

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      {error && (
        <div className="p-2 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-xs text-red-600">{error}</p>
        </div>
      )}

      {/* Keyword */}
      <div className="space-y-1">
        <label className="text-xs font-medium text-gray-700">
          용어
        </label>
        <input
          type="text"
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          placeholder="예: RFC, Lunit, API"
          maxLength={100}
          className="w-full px-3 py-2 bg-purple-50 rounded-lg border-2 border-purple-200 text-sm focus:ring-2 focus:ring-purple-500 focus:border-purple-400 outline-none"
          disabled={isLoading}
          autoFocus
        />
      </div>

      {/* Description */}
      <div className="space-y-1">
        <label className="text-xs font-medium text-gray-700">
          설명 <span className="font-normal text-gray-400">(번역 시 참고)</span>
        </label>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="예: 회사명, 영어는 Lunit, 한국어는 루닛"
          rows={3}
          maxLength={500}
          className="w-full px-3 py-2 bg-purple-50 rounded-lg border-2 border-purple-200 text-sm focus:ring-2 focus:ring-purple-500 focus:border-purple-400 outline-none resize-none"
          disabled={isLoading}
        />
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-1.5 text-sm font-medium text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
        >
          취소
        </button>
        <button
          type="submit"
          disabled={isLoading}
          className="px-3 py-1.5 text-sm font-medium text-white bg-purple-500 hover:bg-purple-600 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5"
        >
          {isLoading && (
            <svg className="animate-spin h-3.5 w-3.5" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
          )}
          {entry ? "수정" : "추가"}
        </button>
      </div>
    </form>
  );
}
