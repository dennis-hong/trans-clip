import { useEffect, useState, useCallback } from "react";
import { createPortal } from "react-dom";
import { useGlossaryStore } from "@/store";
import { Button, Icon, IconButton, Modal, Spinner } from "@/components/common";
import { GlossaryEditor } from "./GlossaryEditor";
import type { GlossaryEntry } from "@/types";

interface GlossaryListProps {
  /**
   * When given, the search/add controls are rendered into this element (the
   * window toolbar) instead of an extra bar above the list.
   */
  toolbarSlot?: HTMLElement | null;
}

export function GlossaryList({ toolbarSlot }: GlossaryListProps) {
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
      if (confirm("이 용어를 삭제하시겠습니까?")) {
        await deleteEntry(id);
      }
    },
    [deleteEntry]
  );

  const handleCancel = useCallback(() => {
    setIsEditorOpen(false);
    setEditingEntry(undefined);
  }, []);

  const controls = (
    <>
      <div className="relative w-[260px] shrink">
        <label htmlFor="glossary-search" className="sr-only">
          용어집 검색
        </label>
        <Icon
          name="search"
          size={14}
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-label-3"
        />
        <input
          id="glossary-search"
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          aria-label="용어집 검색"
          placeholder="용어 검색"
          className="field field-search"
        />
      </div>

      <div className="ml-auto flex shrink-0 items-center gap-3">
        <span className="text-caption tabular-nums text-label-3">{entries.length}개</span>
        <Button variant="primary" tone="purple" icon="plus" onClick={handleAddNew}>
          추가
        </Button>
      </div>
    </>
  );

  return (
    <div className="tone-purple flex h-full flex-col">
      {toolbarSlot ? createPortal(controls, toolbarSlot) : <div className="toolbar">{controls}</div>}

      {/* Terms */}
      <div className="scroll-fade-y min-h-0 flex-1 overflow-y-auto px-4 pb-5 pt-4">
        {isLoading && entries.length === 0 ? (
          <div
            className="flex h-full items-center justify-center gap-2 text-body text-label-3"
            role="status"
            aria-live="polite"
          >
            <Spinner size={16} />
            <span>용어집을 불러오는 중…</span>
          </div>
        ) : error ? (
          <div className="flex h-full items-center justify-center gap-2 text-body text-red-fg">
            <Icon name="warning" size={16} />
            <p>{error}</p>
          </div>
        ) : entries.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <Icon name="book" size={30} strokeWidth={1.3} className="text-label-3" />
            <p className="mt-2.5 text-body font-medium text-label-2">
              {searchQuery ? "검색 결과가 없습니다" : "용어집이 비어있습니다"}
            </p>
            <p className="mt-0.5 text-sub text-label-3">
              번역 품질을 높이려면 자주 쓰는 키워드를 용어집에 추가해 주세요.
            </p>
            {!searchQuery && (
              <Button className="mt-3.5" variant="tinted" tone="purple" icon="plus" onClick={handleAddNew}>
                용어 추가하기
              </Button>
            )}
          </div>
        ) : (
          <ul
            className="grid gap-3"
            style={{ gridTemplateColumns: "repeat(auto-fill, minmax(236px, 1fr))" }}
          >
            {entries.map((entry) => (
              <li key={entry.id} className="flex">
                <GlossaryCard entry={entry} onEdit={handleEdit} onDelete={handleDelete} />
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Editor Modal */}
      <Modal
        isOpen={isEditorOpen}
        onClose={handleCancel}
        title={editingEntry ? "용어 수정" : "새 용어 추가"}
        showCloseButton={false}
      >
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

// Glossary term as a tinted glass note
interface GlossaryCardProps {
  entry: GlossaryEntry;
  onEdit: (entry: GlossaryEntry) => void;
  onDelete: (id: string) => void;
}

function GlossaryCard({ entry, onEdit, onDelete }: GlossaryCardProps) {
  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={`용어 ${entry.keyword} 수정`}
      onClick={() => onEdit(entry)}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) {
          return;
        }
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onEdit(entry);
        }
      }}
      className="group note note-purple min-h-[136px] w-full cursor-default px-3.5 pb-2 pt-3.5"
    >
      <h3 className="truncate text-headline text-label">{entry.keyword}</h3>

      <p className="mt-1.5 line-clamp-3 flex-1 break-keep text-sub leading-[18px] text-label-2 [overflow-wrap:anywhere]">
        {entry.description}
      </p>

      {/* Footer: usage at rest, actions on hover / keyboard focus */}
      <div className="relative mt-1 h-7 shrink-0">
        <div className="absolute inset-0 flex items-center text-caption text-label-3 transition-opacity duration-150 group-focus-within:opacity-0 group-hover:opacity-0">
          사용 {entry.usageCount}회
        </div>

        <div className="absolute -inset-x-1 inset-y-0 flex items-center justify-end gap-0.5 opacity-0 transition-opacity duration-150 group-focus-within:opacity-100 group-hover:opacity-100">
          <IconButton
            icon="square-pencil"
            label="수정"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              onEdit(entry);
            }}
          />
          <IconButton
            icon="trash"
            label="삭제"
            size="sm"
            danger
            onClick={(e) => {
              e.stopPropagation();
              onDelete(entry.id);
            }}
          />
        </div>
      </div>
    </div>
  );
}
