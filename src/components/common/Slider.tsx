import type { CSSProperties } from "react";

interface SliderProps {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  label: string;
  disabled?: boolean;
  className?: string;
}

const THUMB = 20;

export function Slider({ value, min, max, step, onChange, label, disabled, className = "" }: SliderProps) {
  const ratio = max > min ? (value - min) / (max - min) : 0;
  // The thumb's center travels (width - thumb), so offset the filled track to match it.
  const style = {
    "--p": `calc(${THUMB / 2}px + (100% - ${THUMB}px) * ${ratio})`,
  } as CSSProperties;

  return (
    <input
      type="range"
      aria-label={label}
      className={`slider ${className}`}
      min={min}
      max={max}
      step={step}
      value={value}
      disabled={disabled}
      style={style}
      onChange={(event) => onChange(Number(event.target.value))}
    />
  );
}
