import { useEffect, useState, useCallback, useRef } from "react";
import { useGlossaryStore } from "@/store";
import { Modal } from "@/components/common";
import { GlossaryEditor } from "./GlossaryEditor";
import type { GlossaryEntry } from "@/types";

export function GlossaryList() {
  const [searchQuery, setSearchQuery] = useState("");
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<GlossaryEntry | undefined>();
  const scrollRef = useRef<HTMLDivElement>(null);

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

  // Horizontal scroll with mouse wheel
  const handleWheel = (e: React.WheelEvent) => {
    if (scrollRef.current) {
      scrollRef.current.scrollLeft += e.deltaY;
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-gray-200/50">
        {/* Search */}
        <div className="relative flex-1 max-w-xs">
          <svg
            className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
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
            placeholder="용어 검색..."
            className="w-full pl-8 pr-3 py-1.5 bg-white/80 rounded-lg border border-gray-200 text-sm focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none"
          />
        </div>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Entry count */}
        <span className="text-xs text-gray-500 tabular-nums">
          {entries.length}개
        </span>

        {/* Add Button */}
        <button
          onClick={handleAddNew}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-white bg-purple-500 hover:bg-purple-600 rounded-lg transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          추가
        </button>
      </div>

      {/* Card List - 가로 스크롤 */}
      <div
        ref={scrollRef}
        onWheel={handleWheel}
        className="flex-1 flex items-start gap-4 px-4 py-3 overflow-x-auto overflow-y-hidden scroll-smooth"
        style={{
          scrollbarWidth: "thin",
          scrollbarColor: "rgba(156, 163, 175, 0.5) transparent",
        }}
      >
        {isLoading && entries.length === 0 ? (
          <div className="flex items-center justify-center w-full py-8">
            <div className="animate-spin w-6 h-6 border-2 border-purple-500 border-t-transparent rounded-full" />
          </div>
        ) : error ? (
          <div className="flex items-center justify-center w-full py-8">
            <p className="text-sm text-red-500">{error}</p>
          </div>
        ) : entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center w-full py-8 text-center">
            <svg
              className="w-12 h-12 text-gray-300"
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
            <p className="mt-2 text-sm text-gray-500">
              {searchQuery ? "검색 결과가 없습니다" : "용어집이 비어있습니다"}
            </p>
            <p className="text-xs text-gray-400">
              번역 품질 향상을 위해 용어를 추가하세요
            </p>
          </div>
        ) : (
          entries.map((entry) => (
            <GlossaryCard
              key={entry.id}
              entry={entry}
              onEdit={handleEdit}
              onDelete={handleDelete}
            />
          ))
        )}
      </div>

      {/* Editor Modal */}
      <Modal
        isOpen={isEditorOpen}
        onClose={handleCancel}
        title={editingEntry ? "용어 수정" : "새 용어 추가"}
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

// 포스트잇 스타일 용어 카드
interface GlossaryCardProps {
  entry: GlossaryEntry;
  onEdit: (entry: GlossaryEntry) => void;
  onDelete: (id: string) => void;
}

function GlossaryCard({ entry, onEdit, onDelete }: GlossaryCardProps) {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative flex-shrink-0 w-48 h-48 p-3 rounded-lg border-2 cursor-pointer
        transition-all duration-200 ease-out
        bg-purple-100 border-purple-300
        ${isHovered ? "scale-105 shadow-lg -translate-y-1" : "shadow-md"}
      `}
      onClick={() => onEdit(entry)}
    >
      {/* Keyword (Title) */}
      <div className="font-bold text-purple-800 text-base mb-2 truncate">
        {entry.keyword}
      </div>

      {/* Description */}
      <div className="text-xs text-purple-700 overflow-hidden line-clamp-5 leading-relaxed">
        {entry.description}
      </div>

      {/* Footer */}
      <div className="absolute bottom-2 left-3 right-3 flex items-center justify-between">
        <span className="text-[10px] text-purple-600">
          사용 {entry.usageCount}회
        </span>

        {/* Action buttons - visible on hover */}
        <div
          className={`flex gap-0.5 transition-opacity duration-150 ${
            isHovered ? "opacity-100" : "opacity-0"
          }`}
        >
          <button
            onClick={(e) => {
              e.stopPropagation();
              onEdit(entry);
            }}
            className="p-1 rounded hover:bg-purple-200 transition-colors"
            title="수정"
          >
            <svg className="w-3.5 h-3.5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
              />
            </svg>
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete(entry.id);
            }}
            className="p-1 rounded hover:bg-purple-200 transition-colors"
            title="삭제"
          >
            <svg className="w-3.5 h-3.5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
