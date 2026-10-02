import { useState, useCallback, useEffect, useId } from "react";
import { useSettingsStore } from "@/store";
import { Badge, Button, Icon } from "@/components/common";

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
  const inputId = useId();
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
      <div className="w-full">
        <div className="mb-1.5 flex items-center justify-between gap-2">
          <span className="text-sub font-medium text-label">{label}</span>
          <Badge tone="green" icon="check">
            API 키가 설정되었습니다
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <div
            className="field field-sm flex-1 select-none tracking-[0.2em] text-label-3"
            aria-label={`${label} (숨김)`}
          >
            ••••••••••••••••
          </div>
          <Button size="sm" onClick={() => setIsEditing(true)}>
            변경
          </Button>
          <Button size="sm" variant="tinted" tone="red" onClick={handleDelete}>
            삭제
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="w-full">
      <label htmlFor={inputId} className="mb-1.5 block text-sub font-medium text-label">
        {label}
      </label>
      <div className="relative">
        <input
          id={inputId}
          type={showKey ? "text" : "password"}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="API key"
          className="field field-sm pr-9"
          autoComplete="off"
          spellCheck={false}
        />
        <button
          type="button"
          onClick={() => setShowKey(!showKey)}
          className="icon-btn icon-btn-sm absolute right-1 top-1/2 -translate-y-1/2"
          aria-label={showKey ? "API 키 숨기기" : "API 키 보기"}
        >
          <Icon name={showKey ? "eye-off" : "eye"} size={15} />
        </button>
      </div>
      {error && <p className="mt-1.5 text-caption text-red-fg">{error}</p>}
      <p className="mt-1.5 text-caption text-label-3">{description}</p>
      <div className="mt-2.5 flex items-center gap-2">
        <Button
          size="sm"
          variant="primary"
          onClick={handleSave}
          disabled={!apiKey.trim() || isLoading}
          loading={isLoading}
        >
          저장
        </Button>
        {hasApiKey && (
          <Button size="sm" variant="plain" onClick={handleCancel}>
            취소
          </Button>
        )}
      </div>
    </div>
  );
}
