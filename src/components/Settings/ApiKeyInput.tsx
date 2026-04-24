import { useState, useCallback, useEffect } from "react";
import { useSettingsStore } from "@/store";

interface ApiKeyInputProps {
  hasApiKey: boolean;
}

export function ApiKeyInput({ hasApiKey }: ApiKeyInputProps) {
  const [apiKey, setApiKey] = useState("");
  const [isEditing, setIsEditing] = useState(!hasApiKey);
  const [showKey, setShowKey] = useState(false);
  const { setApiKey: saveApiKey, deleteApiKey, isLoading, error } = useSettingsStore();

  // Sync isEditing state when hasApiKey changes
  useEffect(() => {
    if (hasApiKey) {
      setIsEditing(false);
    }
  }, [hasApiKey]);

  const handleSave = useCallback(async () => {
    if (!apiKey.trim()) return;

    const result = await saveApiKey(apiKey);
    if (result.success) {
      setApiKey("");
      setIsEditing(false);
    }
  }, [apiKey, saveApiKey]);

  const handleDelete = useCallback(async () => {
    const success = await deleteApiKey();
    if (success) {
      setIsEditing(true);
    }
  }, [deleteApiKey]);

  const handleCancel = useCallback(() => {
    setApiKey("");
    setIsEditing(false);
  }, []);

  if (!isEditing && hasApiKey) {
    return (
      <div className="space-y-2">
        <label className="block text-xs font-medium text-yellow-700">
          Claude API 키
        </label>
        <div className="flex items-center gap-2">
          <div className="flex-1 px-2 py-1.5 bg-yellow-50 rounded-md border border-yellow-300">
            <span className="text-sm text-yellow-700">
              ••••••••••••••••
            </span>
          </div>
          <button
            onClick={() => setIsEditing(true)}
            className="px-2 py-1 text-xs font-medium text-yellow-700 bg-yellow-200 hover:bg-yellow-300 rounded transition-colors"
          >
            변경
          </button>
          <button
            onClick={handleDelete}
            className="px-2 py-1 text-xs text-yellow-600 hover:bg-yellow-200 rounded transition-colors"
          >
            삭제
          </button>
        </div>
        <p className="text-[10px] text-green-600 flex items-center gap-1">
          <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
          </svg>
          API 키가 설정되었습니다
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <label className="block text-xs font-medium text-yellow-700">
        Claude API 키
      </label>
      <div className="relative">
        <input
          type={showKey ? "text" : "password"}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-ant-... 또는 sk-..."
          className="w-full px-2 py-1.5 pr-8 bg-white rounded-md border border-yellow-300 focus:ring-2 focus:ring-yellow-500 focus:border-yellow-400 outline-none text-sm"
        />
        <button
          type="button"
          onClick={() => setShowKey(!showKey)}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 text-yellow-500 hover:text-yellow-700"
        >
          {showKey ? (
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
            </svg>
          ) : (
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
          )}
        </button>
      </div>
      {error && (
        <p className="text-[10px] text-red-600">{error}</p>
      )}
      <p className="text-[10px] text-yellow-600">
        <a
          href="https://console.anthropic.com"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-600 hover:underline"
        >
          console.anthropic.com
        </a>
        또는 사내 API gateway에서 발급받은 키를 입력하세요
      </p>
      <div className="flex items-center gap-2">
        <button
          onClick={handleSave}
          disabled={!apiKey.trim() || isLoading}
          className="px-3 py-1.5 text-xs font-medium text-white bg-yellow-500 hover:bg-yellow-600 rounded-md disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1"
        >
          {isLoading && (
            <svg className="animate-spin h-3 w-3" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
          )}
          저장
        </button>
        {hasApiKey && (
          <button
            onClick={handleCancel}
            className="px-3 py-1.5 text-xs text-yellow-700 hover:bg-yellow-200 rounded-md transition-colors"
          >
            취소
          </button>
        )}
      </div>
    </div>
  );
}
