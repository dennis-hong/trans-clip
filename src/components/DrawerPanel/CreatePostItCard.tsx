import { useState } from "react";

interface CreatePostItCardProps {
  onClick: () => void;
}

export function CreatePostItCard({ onClick }: CreatePostItCardProps) {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      onClick={onClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative flex-shrink-0 w-48 h-48 p-3 rounded-lg cursor-pointer
        transition-all duration-200 ease-out
        border-2 border-dashed
        ${isHovered
          ? "border-amber-400 bg-amber-50 scale-105 shadow-lg -translate-y-1"
          : "border-gray-300 bg-gray-50 shadow-md"
        }
      `}
      style={{
        transform: isHovered ? "scale(1.05) translateY(-4px)" : "none",
      }}
    >
      {/* Content */}
      <div className="flex flex-col items-center justify-center h-full">
        {/* Plus icon */}
        <div
          className={`
            w-12 h-12 rounded-full flex items-center justify-center mb-3
            transition-colors duration-200
            ${isHovered ? "bg-amber-200" : "bg-gray-200"}
          `}
        >
          <svg
            className={`w-6 h-6 transition-colors duration-200 ${
              isHovered ? "text-amber-600" : "text-gray-500"
            }`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2.5}
              d="M12 4v16m8-8H4"
            />
          </svg>
        </div>

        {/* Label */}
        <span
          className={`text-sm font-medium transition-colors duration-200 ${
            isHovered ? "text-amber-700" : "text-gray-500"
          }`}
        >
          새 메모
        </span>

        {/* Keyboard shortcut hint */}
        <span
          className={`mt-1 text-[10px] font-mono px-1.5 py-0.5 rounded transition-colors duration-200 ${
            isHovered ? "bg-amber-200 text-amber-700" : "bg-gray-200 text-gray-500"
          }`}
        >
          ⌘N
        </span>
      </div>
    </div>
  );
}
