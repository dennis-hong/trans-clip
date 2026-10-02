import type { CSSProperties, ReactNode } from "react";

interface SegmentedOption<T extends string | number> {
  value: T;
  label: ReactNode;
  title?: string;
}

interface SegmentedProps<T extends string | number> {
  value: T;
  options: SegmentedOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
  disabled?: boolean;
  className?: string;
}

/** Segmented control with a sliding selection thumb. */
export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  ariaLabel,
  disabled,
  className = "",
}: SegmentedProps<T>) {
  const selectedIndex = Math.max(
    0,
    options.findIndex((option) => option.value === value)
  );
  const style = { "--n": options.length, "--i": selectedIndex } as CSSProperties;

  return (
    <div role="group" aria-label={ariaLabel} className={`segmented ${className}`} style={style}>
      <span className="segmented-thumb" aria-hidden="true" />
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          className="segmented-item"
          aria-pressed={option.value === value}
          title={option.title}
          disabled={disabled}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
