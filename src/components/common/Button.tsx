import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import { Spinner } from "./Spinner";

export type Tone = "accent" | "purple" | "green" | "red" | "orange" | "yellow";

// Full class names on purpose: Tailwind only keeps classes it can find as literals.
export const TONE_CLASS: Record<Tone, string> = {
  accent: "tone-accent",
  purple: "tone-purple",
  green: "tone-green",
  red: "tone-red",
  orange: "tone-orange",
  yellow: "tone-yellow",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** primary = filled accent, tinted = soft accent, secondary = glass, plain = text only */
  variant?: "primary" | "secondary" | "tinted" | "plain" | "ghost";
  tone?: Tone;
  size?: "sm" | "md" | "lg";
  loading?: boolean;
  icon?: IconName;
  children?: ReactNode;
}

const VARIANT_CLASS = {
  primary: "btn-primary",
  secondary: "",
  tinted: "btn-tinted",
  plain: "btn-plain",
  ghost: "btn-plain",
} as const;

const SIZE_CLASS = {
  sm: "btn-sm",
  md: "",
  lg: "btn-lg",
} as const;

export function Button({
  variant = "secondary",
  tone = "accent",
  size = "md",
  loading = false,
  icon,
  disabled,
  children,
  className = "",
  type = "button",
  ...props
}: ButtonProps) {
  const classes = [
    "btn",
    SIZE_CLASS[size],
    VARIANT_CLASS[variant],
    TONE_CLASS[tone],
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button type={type} className={classes} disabled={disabled || loading} {...props}>
      {loading ? (
        <Spinner size={size === "sm" ? 12 : 14} />
      ) : (
        icon && <Icon name={icon} size={size === "sm" ? 13 : 15} />
      )}
      {children}
    </button>
  );
}
