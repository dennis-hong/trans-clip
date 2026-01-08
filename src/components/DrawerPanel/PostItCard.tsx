import { useCallback, useState } from "react";
import type { ClipboardItem } from "@/types";

interface PostItCardProps {
  item: ClipboardItem;
  color?: string;
  onCopy: (item: ClipboardItem) => void;
  onDelete: (id: string) => void;
  onTogglePin: (id: string) => void;
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
  return COLORS[Math.abs(hash) % COLORS.length];
}

export function PostItCard({ item, color, onCopy, onDelete, onTogglePin }: PostItCardProps) {
  const [isHovered, setIsHovered] = useState(false);
  const cardColor = color || getColorFromId(item.id);

  const handleCopy = useCallback(() => {
    onCopy(item);
  }, [item, onCopy]);

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
    const firstLine = content.split("\n")[0].trim();
    if (firstLine.length <= 30) return firstLine;
    return firstLine.substring(0, 27) + "...";
  };

  return (
    <div
      onClick={handleCopy}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative flex-shrink-0 w-48 h-40 p-3 rounded-lg border-2 cursor-pointer
        transition-all duration-200 ease-out
        ${cardColor}
        ${isHovered ? "scale-105 shadow-lg -translate-y-1" : "shadow-md"}
        ${item.isPinned ? "ring-2 ring-blue-500" : ""}
      `}
      style={{
        transform: isHovered ? "scale(1.05) translateY(-4px)" : "none",
      }}
    >
      {/* Pin indicator */}
      {item.isPinned && (
        <div className="absolute -top-2 -right-2 w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center shadow-md">
          <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
            <path d="M16,12V4H17V2H7V4H8V12L6,14V16H11.2V22H12.8V16H18V14L16,12Z" />
          </svg>
        </div>
      )}

      {/* Title */}
      <div className="font-semibold text-gray-800 text-sm mb-2 truncate">
        {getTitle(item.content)}
      </div>

      {/* Content preview */}
      <div className="text-xs text-gray-600 overflow-hidden line-clamp-4 leading-relaxed">
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
          className={`flex gap-1 transition-opacity duration-150 ${
            isHovered ? "opacity-100" : "opacity-0"
          }`}
        >
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
          <button
            onClick={handleDelete}
            className="p-1 rounded hover:bg-black/10 transition-colors"
            title="삭제"
          >
            <svg className="w-3.5 h-3.5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
