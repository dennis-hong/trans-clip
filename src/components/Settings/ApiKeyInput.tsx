import { useState, useCallback, useEffect } from "react";
import { useSettingsStore } from "@/store";

interface ApiKeyInputProps {
  hasApiKey: boolean;
  label?: string;
  description?: string;
  account?: string;
}

export function ApiKeyInput({
  hasApiKey,
  label = "API 키",
  description = "Provider 또는 custom endpoint에서 발급받은 키를 입력하세요",
  account,
}: ApiKeyInputProps) {
  const [apiKey, setApiKey] = useState("");
  const [isEditing, setIsEditing] = useState(!hasApiKey);
  const [showKey, setShowKey] = useState(false);
  const {
    setApiKey: saveApiKey,
    deleteApiKey,
    setAiApiKey,
    deleteAiApiKey,
    isLoading,
    error,
  } = useSettingsStore();

  // Sync isEditing state when hasApiKey changes
  useEffect(() => {
    if (hasApiKey) {
      setIsEditing(false);
    }
  }, [hasApiKey]);

  const handleSave = useCallback(async () => {
    if (!apiKey.trim()) return;

    const result = account
      ? await setAiApiKey(account, apiKey)
      : await saveApiKey(apiKey);
    if (result.success) {
      setApiKey("");
      setIsEditing(false);
    }
  }, [account, apiKey, saveApiKey, setAiApiKey]);

  const handleDelete = useCallback(async () => {
    const success = account
      ? await deleteAiApiKey(account)
      : await deleteApiKey();
    if (success) {
      setIsEditing(true);
    }
  }, [account, deleteApiKey, deleteAiApiKey]);

  const handleCancel = useCallback(() => {
    setApiKey("");
    setIsEditing(false);
  }, []);

  if (!isEditing && hasApiKey) {
    return (
      <div className="space-y-2">
        <label className="block text-xs font-medium text-slate-700">
          {label}
        </label>
        <div className="flex items-center gap-2">
          <div className="flex-1 rounded-md border border-slate-200 bg-slate-50 px-2 py-1.5">
            <span className="text-sm text-slate-500">
              ••••••••••••••••
            </span>
          </div>
          <button
            onClick={() => setIsEditing(true)}
            className="rounded px-2 py-1 text-xs font-medium text-slate-600 transition-colors hover:bg-slate-100"
          >
            변경
          </button>
          <button
            onClick={handleDelete}
            className="rounded px-2 py-1 text-xs text-red-600 transition-colors hover:bg-red-50"
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
      <label className="block text-xs font-medium text-slate-700">
        {label}
      </label>
      <div className="relative">
        <input
          type={showKey ? "text" : "password"}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="API key"
          className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 pr-8 text-sm outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
        />
        <button
          type="button"
          onClick={() => setShowKey(!showKey)}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 text-slate-400 transition-colors hover:text-slate-700"
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
      <p className="text-[10px] text-slate-500">
        {description}
      </p>
      <div className="flex items-center gap-2">
        <button
          onClick={handleSave}
          disabled={!apiKey.trim() || isLoading}
          className="flex items-center gap-1 rounded-md bg-blue-600 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
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
            className="rounded-md px-3 py-1.5 text-xs text-slate-600 transition-colors hover:bg-slate-100"
          >
            취소
          </button>
        )}
      </div>
    </div>
  );
}
