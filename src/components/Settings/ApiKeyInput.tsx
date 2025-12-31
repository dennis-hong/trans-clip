import { useState, useCallback, useEffect } from "react";
import { Button } from "@/components/common";
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
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
          Claude API Key
        </label>
        <div className="flex items-center gap-2">
          <div className="flex-1 px-3 py-2 bg-gray-100 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <span className="text-sm text-gray-600 dark:text-gray-400">
              ••••••••••••••••
            </span>
          </div>
          <Button variant="secondary" size="sm" onClick={() => setIsEditing(true)}>
            Change
          </Button>
          <Button variant="ghost" size="sm" onClick={handleDelete}>
            Remove
          </Button>
        </div>
        <p className="text-xs text-green-600 dark:text-green-400">
          API key is configured
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
        Claude API Key
      </label>
      <div className="relative">
        <input
          type={showKey ? "text" : "password"}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-ant-..."
          className="w-full px-3 py-2 pr-10 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none text-sm"
        />
        <button
          type="button"
          onClick={() => setShowKey(!showKey)}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
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
        <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
      )}
      <p className="text-xs text-gray-500 dark:text-gray-400">
        Get your API key from{" "}
        <a
          href="https://console.anthropic.com"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-600 hover:underline"
        >
          console.anthropic.com
        </a>
      </p>
      <div className="flex items-center gap-2">
        <Button
          variant="primary"
          size="sm"
          onClick={handleSave}
          loading={isLoading}
          disabled={!apiKey.trim() || isLoading}
        >
          Save
        </Button>
        {hasApiKey && (
          <Button variant="ghost" size="sm" onClick={handleCancel}>
            Cancel
          </Button>
        )}
      </div>
    </div>
  );
}
