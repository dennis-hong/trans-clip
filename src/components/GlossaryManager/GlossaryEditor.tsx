import { useState, useCallback, useId } from "react";
import { Button, Icon } from "@/components/common";
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
  const keywordId = useId();
  const descriptionId = useId();

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
    <form onSubmit={handleSubmit} className="space-y-3.5">
      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-control bg-red/10 px-3 py-2 text-sub text-red-fg"
        >
          <Icon name="warning" size={14} className="mt-px shrink-0" />
          <p>{error}</p>
        </div>
      )}

      {/* Keyword */}
      <div>
        <label htmlFor={keywordId} className="mb-1.5 block text-sub font-medium text-label">
          용어
        </label>
        <input
          id={keywordId}
          type="text"
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          placeholder="예: RFC, Lunit, API"
          maxLength={100}
          className="field"
          disabled={isLoading}
          autoFocus
        />
      </div>

      {/* Description */}
      <div>
        <label htmlFor={descriptionId} className="mb-1.5 block text-sub font-medium text-label">
          설명 <span className="font-normal text-label-3">(번역 시 참고)</span>
        </label>
        <textarea
          id={descriptionId}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="예: 회사명, 영어는 Lunit, 한국어는 루닛"
          rows={3}
          maxLength={500}
          className="field resize-none"
          disabled={isLoading}
        />
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-2 pt-1">
        <Button onClick={onCancel}>취소</Button>
        <Button type="submit" variant="primary" tone="purple" loading={isLoading}>
          {entry ? "수정" : "추가"}
        </Button>
      </div>
    </form>
  );
}
