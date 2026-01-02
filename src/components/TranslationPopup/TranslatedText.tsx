
interface TranslatedTextProps {
  text: string;
  language?: string;
  isLoading?: boolean;
  error?: string;
}

export function TranslatedText({
  text,
  language,
  isLoading,
  error,
}: TranslatedTextProps) {
  const languageLabel = language === "ko" ? "Korean" : "English";

  return (
    <div className="flex flex-col h-full space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          Translation ({languageLabel})
        </span>
      </div>
      <div className="flex-1 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 overflow-y-auto min-h-[120px] max-h-[300px]">
        {isLoading ? (
          <div className="flex items-center space-x-2">
            <svg
              className="animate-spin h-4 w-4 text-blue-500"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            <span className="text-sm text-blue-600 dark:text-blue-400">
              Translating...
            </span>
          </div>
        ) : error ? (
          <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
        ) : (
          <p className="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap break-words">
            {text}
          </p>
        )}
      </div>
    </div>
  );
}
