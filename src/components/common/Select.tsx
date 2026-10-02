import { forwardRef, type SelectHTMLAttributes } from "react";
import { Icon } from "./Icon";

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  /** Stretch to the width of the container. */
  block?: boolean;
  wrapperClassName?: string;
}

/**
 * Pop-up button. It stays a real <select> (native menu, keyboard and
 * accessibility behavior); only the closed state is restyled.
 */
export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  { block = false, className = "", wrapperClassName = "", children, ...props },
  ref
) {
  return (
    <span className={`select-wrap ${block ? "select-block" : ""} ${wrapperClassName}`}>
      <select ref={ref} className={`select ${className}`} {...props}>
        {children}
      </select>
      <Icon name="chevron-up-down" size={12} strokeWidth={2} className="select-chevron" />
    </span>
  );
});
