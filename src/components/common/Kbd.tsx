import type { HTMLAttributes } from "react";

/** A small keycap, used for shortcut hints (⌘↵, ESC, 1–9 …). */
export function Kbd({ children, className = "", ...rest }: HTMLAttributes<HTMLElement>) {
  return (
    <kbd className={`kbd ${className}`} {...rest}>
      {children}
    </kbd>
  );
}
