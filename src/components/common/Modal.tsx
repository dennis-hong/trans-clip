import React, { useEffect, useCallback } from "react";
import { IconButton } from "./IconButton";

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  showCloseButton?: boolean;
}

/** Sheet-style dialog: dimmed backdrop, centered glass sheet. */
export function Modal({
  isOpen,
  onClose,
  title,
  children,
  showCloseButton = true,
}: ModalProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    if (isOpen) {
      document.addEventListener("keydown", handleKeyDown);
      return () => document.removeEventListener("keydown", handleKeyDown);
    }
  }, [isOpen, handleKeyDown]);

  if (!isOpen) return null;

  const hasHeader = title || showCloseButton;

  return (
    <div className="animate-fade-in fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Backdrop: matches the window's rounded shape so corners stay clean */}
      <div
        className="absolute inset-0 rounded-panel bg-black/25 backdrop-blur-[3px]"
        onClick={onClose}
      />

      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="sheet animate-pop-in relative w-full max-w-[400px]"
      >
        {hasHeader && (
          <div className="flex items-center justify-between gap-3 px-5 pt-4">
            {title ? <h2 className="text-title text-label">{title}</h2> : <span />}
            {showCloseButton && <IconButton icon="xmark" label="닫기" size="sm" onClick={onClose} />}
          </div>
        )}

        <div className="px-5 pb-5 pt-3">{children}</div>
      </div>
    </div>
  );
}
