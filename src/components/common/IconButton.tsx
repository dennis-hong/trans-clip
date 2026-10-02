import type { ButtonHTMLAttributes } from "react";
import { Icon, type IconName } from "./Icon";

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: IconName;
  /** Accessible name, also used as the tooltip unless `title` is given. */
  label: string;
  size?: "sm" | "md";
  variant?: "plain" | "glass";
  danger?: boolean;
  iconSize?: number;
}

export function IconButton({
  icon,
  label,
  size = "md",
  variant = "plain",
  danger = false,
  iconSize,
  className = "",
  title,
  type = "button",
  ...props
}: IconButtonProps) {
  const classes = [
    "icon-btn",
    size === "sm" ? "icon-btn-sm" : "",
    variant === "glass" ? "icon-btn-glass" : "",
    danger ? "icon-btn-danger" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button type={type} className={classes} aria-label={label} title={title ?? label} {...props}>
      <Icon name={icon} size={iconSize ?? (size === "sm" ? 14 : 16)} />
    </button>
  );
}
