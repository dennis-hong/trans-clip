import { memo, useCallback } from "react";
import { Icon, IconButton, Kbd } from "@/components/common";
import type { ClipboardItem } from "@/types";

interface PostItCardProps {
  item: ClipboardItem;
  index?: number;
  /** Optional note color class (note-yellow, note-blue, …); defaults to a stable color per item. */
  color?: string;
  onCopy: (item: ClipboardItem) => void;
  onPaste?: (item: ClipboardItem) => void;
  onDelete: (id: string) => void;
  onTogglePin: (id: string) => void;
  onTranslate?: (item: ClipboardItem) => void;
  onPolish?: (item: ClipboardItem) => void;
  onEdit?: (item: ClipboardItem) => void;
  showPasteButton?: boolean;
}

/** Width of a note in the drawer strip (px). Keep in sync with keyboard scrolling. */
export const POSTIT_CARD_WIDTH = 204;

const COLORS = [
  "note-yellow",
  "note-blue",
  "note-green",
  "note-pink",
  "note-purple",
  "note-orange",
];

function getColorFromId(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = id.charCodeAt(i) + ((hash << 5) - hash);
  }
  const colorIndex = Math.abs(hash) % COLORS.length;
  return COLORS[colorIndex] || "note-yellow";
}

function formatTime(dateStr: string) {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return "방금";
  if (diffMins < 60) return `${diffMins}분`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}시간`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}일`;
}

// Extract title from content (first line or first 30 chars)
function getTitle(content: string): string {
  const firstLine = (content.split("\n")[0] ?? "").trim();
  if (firstLine.length <= 30) return firstLine;
  return firstLine.substring(0, 27) + "...";
}

export const PostItCard = memo(function PostItCard({ item, index, color, onCopy, onPaste, onDelete, onTogglePin, onTranslate, onPolish, onEdit, showPasteButton }: PostItCardProps) {
  const cardColor = color || getColorFromId(item.id);

  const handleCopy = useCallback(() => {
    onCopy(item);
  }, [item, onCopy]);

  const handlePaste = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onPaste) {
        onPaste(item);
      }
    },
    [item, onPaste]
  );

  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onDelete(item.id);
    },
    [item.id, onDelete]
  );

  const handleTogglePin = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onTogglePin(item.id);
    },
    [item.id, onTogglePin]
  );

  const handleTranslate = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onTranslate) {
        onTranslate(item);
      }
    },
    [item, onTranslate]
  );

  const handlePolish = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onPolish) {
        onPolish(item);
      }
    },
    [item, onPolish]
  );

  const handleEdit = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onEdit) {
        onEdit(item);
      }
    },
    [item, onEdit]
  );

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onEdit) {
        onEdit(item);
      }
    },
    [item, onEdit]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.target !== e.currentTarget) {
        return;
      }
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onCopy(item);
      }
    },
    [item, onCopy]
  );

  return (
    <div
      onClick={handleCopy}
      onDoubleClick={handleDoubleClick}
      onKeyDown={handleKeyDown}
      role="button"
      tabIndex={0}
      aria-label={`메모 ${getTitle(item.content)} 복사`}
      className={`group note ${cardColor} ${item.isPinned ? "note-pinned" : ""} h-full cursor-default px-3.5 pb-2 pt-3.5`}
      style={{ width: POSTIT_CARD_WIDTH }}
    >
      {/* Title row: quick-select key, first line, pin state */}
      <div className="flex items-start gap-2">
        {index !== undefined && index < 9 && (
          <Kbd className="mt-px" aria-hidden="true">
            {index + 1}
          </Kbd>
        )}
        <h3 className="min-w-0 flex-1 truncate text-headline text-label">
          {getTitle(item.content)}
        </h3>
        {item.isPinned && (
          <Icon name="pin-fill" size={13} className="mt-0.5 shrink-0 text-accent" />
        )}
      </div>

      {/* Content preview */}
      <p className="mt-2 line-clamp-5 flex-1 overflow-hidden break-keep text-sub leading-[18px] text-label-2 [overflow-wrap:anywhere]">
        {item.contentPreview}
      </p>

      {/* Footer: meta at rest, actions on hover / keyboard focus */}
      <div className="relative mt-1 h-7 shrink-0">
        <div className="absolute inset-0 flex items-center justify-between text-caption text-label-3 transition-opacity duration-150 group-focus-within:opacity-0 group-hover:opacity-0">
          <span className="flex items-center gap-1.5">
            {formatTime(item.copiedAt)}
            {item.updatedAt && <span title={`편집됨: ${item.updatedAt}`}>편집됨</span>}
          </span>
          {item.metadata && <span className="tabular-nums">{item.metadata.characterCount}자</span>}
        </div>

        <div className="absolute -inset-x-1 inset-y-0 flex items-center justify-between opacity-0 transition-opacity duration-150 group-focus-within:opacity-100 group-hover:opacity-100">
          {onTranslate && (
            <IconButton
              icon="translate"
              label="번역"
              size="sm"
              className="hover:text-accent-fg"
              onClick={handleTranslate}
            />
          )}
          {onPolish && (
            <IconButton
              icon="sparkles"
              label="다듬기"
              size="sm"
              className="hover:text-purple-fg"
              onClick={handlePolish}
            />
          )}
          {onEdit && <IconButton icon="square-pencil" label="편집" size="sm" onClick={handleEdit} />}
          {showPasteButton && onPaste && (
            <IconButton icon="clipboard" label="붙여넣기" size="sm" onClick={handlePaste} />
          )}
          <IconButton
            icon={item.isPinned ? "pin-fill" : "pin"}
            label={item.isPinned ? "고정 해제" : "고정"}
            size="sm"
            className={item.isPinned ? "text-accent hover:text-accent" : ""}
            onClick={handleTogglePin}
          />
          <IconButton icon="trash" label="삭제" size="sm" danger onClick={handleDelete} />
        </div>
      </div>
    </div>
  );
});
