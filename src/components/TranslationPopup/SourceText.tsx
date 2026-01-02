
interface SourceTextProps {
  text: string;
  language?: string;
}

export function SourceText({ text, language }: SourceTextProps) {
  const languageLabel = language === "ko" ? "Korean" : "English";

  return (
    <div className="flex flex-col h-full space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          Original ({languageLabel})
        </span>
      </div>
      <div className="flex-1 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-y-auto min-h-[120px] max-h-[300px]">
        <p className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap break-words">
          {text}
        </p>
      </div>
    </div>
  );
}
