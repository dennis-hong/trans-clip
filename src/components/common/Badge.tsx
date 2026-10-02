import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import { TONE_CLASS, type Tone } from "./Button";

interface BadgeProps {
  children: ReactNode;
  /** Tinted by a feature/status color; omit for a neutral capsule. */
  tone?: Tone;
  icon?: IconName;
  className?: string;
}

export function Badge({ children, tone, icon, className = "" }: BadgeProps) {
  const toneClass = tone ? TONE_CLASS[tone] : "badge-neutral";
  return (
    <span className={`badge ${toneClass} ${className}`}>
      {icon && <Icon name={icon} size={11} strokeWidth={2} />}
      {children}
    </span>
  );
}
