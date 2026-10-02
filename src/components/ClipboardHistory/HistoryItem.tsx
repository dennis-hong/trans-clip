import { useCallback } from "react";
import { Icon, IconButton } from "@/components/common";
import type { ClipboardItem } from "@/types";

interface HistoryItemProps {
  item: ClipboardItem;
  onCopy: (item: ClipboardItem) => void;
  onDelete: (id: string) => void;
  onTogglePin: (id: string) => void;
}

export function HistoryItem({ item, onCopy, onDelete, onTogglePin }: HistoryItemProps) {
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

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;

    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;

    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
  };

  return (
    <div
      onClick={handleCopy}
      className="group cursor-default rounded-pane bg-surface p-3 shadow-[inset_0_0_0_0.5px_var(--separator)] transition-colors hover:bg-surface-2"
    >
      <div className="flex items-start gap-2">
        {/* Pin indicator */}
        {item.isPinned && (
          <span className="mt-0.5 flex-shrink-0">
            <Icon name="pin-fill" size={14} className="text-accent" />
          </span>
        )}

        {/* Content */}
        <div className="min-w-0 flex-1">
          <p className="truncate text-body text-label">{item.contentPreview}</p>
          <div className="mt-1 flex items-center gap-2 text-caption text-label-3">
            <span>{formatTime(item.copiedAt)}</span>
            {item.sourceApp && (
              <>
                <span className="text-label-4">•</span>
                <span>{item.sourceApp}</span>
              </>
            )}
            {item.metadata && (
              <>
                <span className="text-label-4">•</span>
                <span>{item.metadata.characterCount} chars</span>
              </>
            )}
          </div>
        </div>

        {/* Actions */}
        <div className="flex flex-shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
          <IconButton
            icon={item.isPinned ? "pin-fill" : "pin"}
            label={item.isPinned ? "Unpin" : "Pin"}
            size="sm"
            onClick={handleTogglePin}
          />
          <IconButton icon="trash" label="Delete" size="sm" danger onClick={handleDelete} />
        </div>
      </div>
    </div>
  );
}
