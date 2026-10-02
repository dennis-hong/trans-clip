interface SpinnerProps {
  size?: number;
  className?: string;
}

const SPOKES = Array.from({ length: 12 }, (_, index) => index);

/** Twelve-spoke indeterminate indicator, modeled on NSProgressIndicator's spinner. */
export function Spinner({ size = 16, className = "" }: SpinnerProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      className={`animate-spinner ${className}`}
      aria-hidden="true"
      focusable="false"
    >
      {SPOKES.map((index) => (
        <line
          key={index}
          x1="12"
          y1="3"
          x2="12"
          y2="7.2"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          opacity={0.18 + (index / 11) * 0.82}
          transform={`rotate(${index * 30} 12 12)`}
        />
      ))}
    </svg>
  );
}
