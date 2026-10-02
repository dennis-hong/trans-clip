import type { ReactElement, SVGProps } from "react";

/**
 * Small line-icon set drawn on a 24pt grid with round caps and a medium stroke,
 * so the glyphs read like SF Symbols ("regular" weight) instead of web icons.
 */
const ICONS = {
  "chevron-left": <path d="M14.5 5.5L8 12l6.5 6.5" />,
  "chevron-right": <path d="M9.5 5.5L16 12l-6.5 6.5" />,
  "chevron-down": <path d="M5.5 9.5L12 16l6.5-6.5" />,
  "chevron-up-down": <path d="M8 9.5l4-4 4 4M8 14.5l4 4 4-4" />,
  xmark: <path d="M6.5 6.5l11 11M17.5 6.5l-11 11" />,
  plus: <path d="M12 5.5v13M5.5 12h13" />,
  check: <path d="M5.5 12.5l4.2 4.3 8.8-9.3" />,
  search: (
    <>
      <circle cx="10.5" cy="10.5" r="6.2" />
      <path d="M15.2 15.2l4.6 4.6" />
    </>
  ),
  gear: (
    <>
      <path d="M10.32 5L10.83 2.77H13.17L13.68 5L15.76 5.86L17.7 4.65L19.35 6.3L18.14 8.24L19 10.32L21.23 10.83V13.17L19 13.68L18.14 15.76L19.35 17.7L17.7 19.35L15.76 18.14L13.68 19L13.17 21.23H10.83L10.32 19L8.24 18.14L6.3 19.35L4.65 17.7L5.86 15.76L5 13.68L2.77 13.17V10.83L5 10.32L5.86 8.24L4.65 6.3L6.3 4.65L8.24 5.86Z" />
      <circle cx="12" cy="12" r="2.9" />
    </>
  ),
  book: (
    <path d="M12 6.8C10.6 5.6 8.6 5 6.4 5H4.5v12.8h1.9c2.2 0 4.2.6 5.6 1.8 1.4-1.2 3.4-1.8 5.6-1.8h1.9V5h-1.9c-2.2 0-4.2.6-5.6 1.8zM12 6.8v12.8" />
  ),
  translate: (
    <path d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
  ),
  sparkles: (
    <>
      <path d="M10.2 7.2c.5 3.7 2.5 5.7 6.2 6.4-3.7.6-5.7 2.6-6.2 6.4-.5-3.8-2.5-5.8-6.2-6.4 3.7-.7 5.7-2.7 6.2-6.4z" />
      <path d="M18.2 3.2c.2 1.6.9 2.4 2.6 2.6-1.7.2-2.4 1-2.6 2.6-.2-1.6-.9-2.4-2.6-2.6 1.7-.2 2.4-1 2.6-2.6z" />
    </>
  ),
  "square-pencil": (
    <>
      <path d="M11 5H7.5A2.5 2.5 0 005 7.5v9A2.5 2.5 0 007.5 19h9a2.5 2.5 0 002.5-2.5V13" />
      <path d="M18.4 4.6a1.9 1.9 0 012.7 2.7l-7.6 7.7-3.4.8.8-3.4 7.5-7.8z" />
    </>
  ),
  clipboard: (
    <>
      <path d="M9 5.5H7.6A2.6 2.6 0 005 8.1v9.3A2.6 2.6 0 007.6 20h8.8a2.6 2.6 0 002.6-2.6V8.1a2.6 2.6 0 00-2.6-2.6H15" />
      <rect x="9" y="3.5" width="6" height="4" rx="1.4" />
    </>
  ),
  copy: (
    <>
      <rect x="8.5" y="8.5" width="11" height="11.5" rx="2.4" />
      <path d="M15.5 8.5V6.9a2.4 2.4 0 00-2.4-2.4H6.9a2.4 2.4 0 00-2.4 2.4v6.2a2.4 2.4 0 002.4 2.4h1.6" />
    </>
  ),
  pin: (
    <g transform="rotate(40 12 12)">
      <path d="M9.2 3.8h5.6l-.8 4.6c1.7.9 2.8 2.5 2.8 4.4v.6H7.2v-.6c0-1.9 1.1-3.5 2.8-4.4L9.2 3.8zM12 13.4v6.8" />
    </g>
  ),
  "pin-fill": (
    <g transform="rotate(40 12 12)">
      <path d="M9.2 3.8h5.6l-.8 4.6c1.7.9 2.8 2.5 2.8 4.4v.6H7.2v-.6c0-1.9 1.1-3.5 2.8-4.4L9.2 3.8z" fill="currentColor" />
      <path d="M12 13.4v6.8" />
    </g>
  ),
  trash: (
    <path d="M5 7h14M9.5 7V5.2A1.2 1.2 0 0110.7 4h2.6a1.2 1.2 0 011.2 1.2V7M6.8 7l.7 11.2A2 2 0 009.5 20h5a2 2 0 002-1.8L17.2 7M10.2 11v5.2M13.8 11v5.2" />
  ),
  warning: <path d="M12 4.5L20.6 19a.9.9 0 01-.8 1.4H4.2a.9.9 0 01-.8-1.4L12 4.5zM12 10v4M12 17.1v.01" />,
  display: (
    <>
      <rect x="3.5" y="4.5" width="17" height="11.5" rx="2.4" />
      <path d="M8.5 19.5h7M12 16v3.5" />
    </>
  ),
  keyboard: (
    <>
      <rect x="2.8" y="6" width="18.4" height="12" rx="2.8" />
      <path d="M6.7 10h.01M10 10h.01M13.3 10h.01M16.6 10h.01M7.8 14h8.4" />
    </>
  ),
  "arrow-right": <path d="M4.5 12h15M14 6.5l5.5 5.5-5.5 5.5" />,
  "external-link": (
    <path d="M10.5 5.5h-3A2.5 2.5 0 005 8v8.5A2.5 2.5 0 007.5 19H16a2.5 2.5 0 002.5-2.5v-3M14 5h5v5M19 5l-8 8" />
  ),
  eye: (
    <>
      <path d="M2.8 12S6.1 5.8 12 5.8 21.2 12 21.2 12 17.9 18.2 12 18.2 2.8 12 2.8 12z" />
      <circle cx="12" cy="12" r="2.8" />
    </>
  ),
  "eye-off": (
    <path d="M9.9 6.1A9 9 0 0112 5.8c5.9 0 9.2 6.2 9.2 6.2a15.6 15.6 0 01-2.5 3.3M6.4 7.8A15.6 15.6 0 002.8 12S6.1 18.2 12 18.2a9.3 9.3 0 004-.9M9.9 9.9a2.8 2.8 0 003.9 3.9M4 4l16 16" />
  ),
  "check-circle-fill": (
    <>
      <circle cx="12" cy="12" r="10" fill="currentColor" stroke="none" />
      <path d="M7.7 12.4l3 3 5.6-6.2" stroke="#fff" strokeWidth={2} />
    </>
  ),
  "xmark-circle-fill": (
    <>
      <circle cx="12" cy="12" r="10" fill="currentColor" stroke="none" />
      <path d="M8.8 8.8l6.4 6.4M15.2 8.8l-6.4 6.4" stroke="#fff" strokeWidth={2} />
    </>
  ),
  "info-circle-fill": (
    <>
      <circle cx="12" cy="12" r="10" fill="currentColor" stroke="none" />
      <path d="M12 11v5.2M12 7.9v.01" stroke="#fff" strokeWidth={2} />
    </>
  ),
} satisfies Record<string, ReactElement>;

export type IconName = keyof typeof ICONS;

interface IconProps extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IconName;
  size?: number;
  strokeWidth?: number;
}

export function Icon({ name, size = 16, strokeWidth = 1.7, className, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
      {...rest}
    >
      {ICONS[name]}
    </svg>
  );
}
