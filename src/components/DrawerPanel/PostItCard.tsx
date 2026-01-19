import { useCallback, useState } from "react";
import type { ClipboardItem } from "@/types";

interface PostItCardProps {
  item: ClipboardItem;
  index?: number;
  color?: string;
  onCopy: (item: ClipboardItem) => void;
  onPaste?: (item: ClipboardItem) => void;
  onDelete: (id: string) => void;
  onTogglePin: (id: string) => void;
  onTranslate?: (item: ClipboardItem) => void;
  onPolish?: (item: ClipboardItem) => void;
  showPasteButton?: boolean;
}

const COLORS = [
  "bg-yellow-100 border-yellow-300",
  "bg-blue-100 border-blue-300",
  "bg-green-100 border-green-300",
  "bg-pink-100 border-pink-300",
  "bg-purple-100 border-purple-300",
  "bg-orange-100 border-orange-300",
];

function getColorFromId(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = id.charCodeAt(i) + ((hash << 5) - hash);
  }
  const colorIndex = Math.abs(hash) % COLORS.length;
  return COLORS[colorIndex] || "bg-yellow-100 border-yellow-300";
}

export function PostItCard({ item, index, color, onCopy, onPaste, onDelete, onTogglePin, onTranslate, onPolish, showPasteButton }: PostItCardProps) {
  const [isHovered, setIsHovered] = useState(false);
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

  const formatTime = (dateStr: string) => {
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
  };

  // Extract title from content (first line or first 20 chars)
  const getTitle = (content: string): string => {
    const firstLine = (content.split("\n")[0] ?? "").trim();
    if (firstLine.length <= 30) return firstLine;
    return firstLine.substring(0, 27) + "...";
  };

  return (
    <div
      onClick={handleCopy}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative flex-shrink-0 w-48 h-48 p-3 rounded-lg border-2 cursor-pointer
        transition-all duration-200 ease-out
        ${cardColor}
        ${isHovered ? "scale-105 shadow-lg -translate-y-1" : "shadow-md"}
        ${item.isPinned ? "ring-2 ring-blue-500" : ""}
      `}
      style={{
        transform: isHovered ? "scale(1.05) translateY(-4px)" : "none",
      }}
    >
      {/* Quick select number badge (1-9) */}
      {index !== undefined && index < 9 && (
        <div className="absolute -top-2 -left-2 w-5 h-5 bg-gray-700 text-white rounded-full flex items-center justify-center shadow-md text-xs font-bold">
          {index + 1}
        </div>
      )}

      {/* Top right area: Pin indicator and Delete button */}
      <div className="absolute -top-2 -right-2 flex items-center gap-1">
        {/* Delete button - visible on hover */}
        <button
          onClick={handleDelete}
          className={`w-5 h-5 bg-red-500 hover:bg-red-600 text-white rounded-full flex items-center justify-center shadow-md transition-all duration-150 ${
            isHovered ? "opacity-100 scale-100" : "opacity-0 scale-75"
          }`}
          title="삭제"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2.5}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
        {/* Pin indicator */}
        {item.isPinned && (
          <div className="w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center shadow-md">
            <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
              <path d="M16,12V4H17V2H7V4H8V12L6,14V16H11.2V22H12.8V16H18V14L16,12Z" />
            </svg>
          </div>
        )}
      </div>

      {/* Title */}
      <div className="font-semibold text-gray-800 text-sm mb-2 truncate">
        {getTitle(item.content)}
      </div>

      {/* Content preview */}
      <div className="text-xs text-gray-600 overflow-hidden line-clamp-5 leading-relaxed">
        {item.contentPreview}
      </div>

      {/* Footer */}
      <div className="absolute bottom-2 left-3 right-3 flex items-center justify-between">
        <span className="text-[10px] text-gray-500">
          {formatTime(item.copiedAt)}
          {item.metadata && ` · ${item.metadata.characterCount}자`}
        </span>

        {/* Action buttons - visible on hover */}
        <div
          className={`flex gap-0.5 transition-opacity duration-150 ${
            isHovered ? "opacity-100" : "opacity-0"
          }`}
        >
          {/* Translate button */}
          {onTranslate && (
            <button
              onClick={handleTranslate}
              className="p-1 rounded hover:bg-black/10 transition-colors"
              title="번역"
            >
              <svg className="w-3.5 h-3.5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
                />
              </svg>
            </button>
          )}
          {/* Polish button */}
          {onPolish && (
            <button
              onClick={handlePolish}
              className="p-1 rounded hover:bg-black/10 transition-colors"
              title="다듬기"
            >
              <svg className="w-3.5 h-3.5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
                />
              </svg>
            </button>
          )}
          {/* Paste button - only shown in stealth mode */}
          {showPasteButton && onPaste && (
            <button
              onClick={handlePaste}
              className="p-1 rounded hover:bg-black/10 transition-colors"
              title="붙여넣기"
            >
              <svg className="w-3.5 h-3.5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                />
              </svg>
            </button>
          )}
          <button
            onClick={handleTogglePin}
            className="p-1 rounded hover:bg-black/10 transition-colors"
            title={item.isPinned ? "고정 해제" : "고정"}
          >
            <svg
              className={`w-3.5 h-3.5 ${item.isPinned ? "text-blue-600" : "text-gray-500"}`}
              fill={item.isPinned ? "currentColor" : "none"}
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
