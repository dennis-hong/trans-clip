import { useEffect, useState } from "react";
import { Icon } from "./Icon";

export interface ToastProps {
  message: string;
  type?: "success" | "error" | "info";
  duration?: number;
  onClose: () => void;
}

const ICON = {
  success: { name: "check-circle-fill", color: "text-green" },
  error: { name: "xmark-circle-fill", color: "text-red" },
  info: { name: "info-circle-fill", color: "text-accent" },
} as const;

export function Toast({ message, type = "success", duration = 2000, onClose }: ToastProps) {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    // Trigger enter animation
    requestAnimationFrame(() => setIsVisible(true));

    const timer = setTimeout(() => {
      setIsVisible(false);
      setTimeout(onClose, 200); // Wait for exit animation
    }, duration);

    return () => clearTimeout(timer);
  }, [duration, onClose]);

  const icon = ICON[type];

  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className={`popover pointer-events-none fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 rounded-full py-2 pl-3 pr-4 text-body font-medium text-label transition-all duration-200 ease-out ${
        isVisible ? "translate-y-0 scale-100 opacity-100" : "translate-y-2 scale-95 opacity-0"
      }`}
    >
      <Icon name={icon.name} size={18} className={icon.color} />
      <span>{message}</span>
    </div>
  );
}
