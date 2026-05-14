import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { useSettingsStore, useUpdateStore } from "@/store";
import { ApiKeyInput } from "./ApiKeyInput";
import {
  CUSTOM_ENDPOINT_API_KEY_ACCOUNT,
  DEFAULT_MODEL_PROFILE_ID,
  PROVIDER_DEFAULT_ENDPOINTS,
  formatModelProfileOption,
  providerLabel,
} from "@/types";
import type {
  AiModelProfile,
  AiProviderConfig,
  EndpointMode,
  PermissionStatus,
  ProviderKind,
} from "@/types";

function providerKeyAccount(provider: AiProviderConfig) {
  return provider.endpointMode === "custom"
    ? CUSTOM_ENDPOINT_API_KEY_ACCOUNT
    : `provider:${provider.id}`;
}

export function SettingsPanel() {
  const {
    settings,
    fetchSettings,
    updateSettings,
    updateAiProviderConfig,
    addAiModelProfile,
    updateAiModelProfile,
    deleteAiModelProfile,
    fetchApiKeyStatus,
    fetchAiApiKeyStatus,
    isLoading,
    error,
    aiApiKeyStatus,
  } = useSettingsStore();
  const {
    currentVersion,
    latestVersion,
    hasUpdate,
    isChecking: isCheckingUpdate,
    isDownloading,
    progress,
    error: updateError,
    initCurrentVersion,
    checkForUpdate,
    installUpdate,
    dismissUpdate,
    clearError: clearUpdateError,
  } = useUpdateStore();

  const [accessibilityGranted, setAccessibilityGranted] = useState<boolean | null>(null);
  const [providerBaseUrls, setProviderBaseUrls] = useState<Record<string, string>>({});
  const [newModelProviderId, setNewModelProviderId] = useState("");
  const [newModelDisplayName, setNewModelDisplayName] = useState("");
  const [newModelId, setNewModelId] = useState("");
  const [editingModelId, setEditingModelId] = useState<string | null>(null);
  const [editModelProviderId, setEditModelProviderId] = useState("");
  const [editModelDisplayName, setEditModelDisplayName] = useState("");
  const [editModelId, setEditModelId] = useState("");
  const [isAiAdvancedOpen, setIsAiAdvancedOpen] = useState(false);
  const accessibilityCheckTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isMountedRef = useRef(true);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
      if (accessibilityCheckTimeoutRef.current) {
        clearTimeout(accessibilityCheckTimeoutRef.current);
      }
    };
  }, []);

  const checkAccessibilityStatus = async (): Promise<boolean> => {
    try {
      const status = await invoke<PermissionStatus>("check_accessibility_permission");
      if (isMountedRef.current) {
        setAccessibilityGranted(status.granted);
      }
      return status.granted;
    } catch (err) {
      console.error("Failed to check accessibility:", err);
      return false;
    }
  };

  useEffect(() => {
    fetchSettings();
    fetchApiKeyStatus();
    checkAccessibilityStatus();
    void initCurrentVersion();
  }, [fetchSettings, fetchApiKeyStatus, initCurrentVersion]);

  useEffect(() => {
    if (!settings) {
      return;
    }
    setProviderBaseUrls(Object.fromEntries(
      settings.aiProviderConfigs.map((provider) => [provider.id, provider.baseUrl])
    ));
    if (!newModelProviderId && settings.aiProviderConfigs[0]) {
      setNewModelProviderId(settings.aiProviderConfigs[0].id);
    }
    for (const provider of settings.aiProviderConfigs) {
      void fetchAiApiKeyStatus(providerKeyAccount(provider));
    }
  }, [fetchAiApiKeyStatus, newModelProviderId, settings]);

  const handleModelChange = (modelProfileId: string) => {
    updateSettings({ preferredModelProfileId: modelProfileId });
  };

  const handleProviderBaseUrlSave = (providerId: string) => {
    const provider = settings?.aiProviderConfigs.find((item) => item.id === providerId);
    if (!provider) {
      return;
    }
    const nextBaseUrl = (providerBaseUrls[providerId] ?? "").trim();
    if (nextBaseUrl === provider.baseUrl) {
      return;
    }
    updateAiProviderConfig({
      id: provider.id,
      endpointMode: provider.endpointMode,
      baseUrl: nextBaseUrl,
      enabled: provider.enabled,
    });
  };

  const handleProviderEndpointModeChange = (providerId: string, endpointMode: EndpointMode) => {
    const provider = settings?.aiProviderConfigs.find((item) => item.id === providerId);
    if (!provider) {
      return;
    }
    const baseUrl = endpointMode === "public"
      ? PROVIDER_DEFAULT_ENDPOINTS[provider.providerKind]
      : providerBaseUrls[providerId] || provider.baseUrl;
    setProviderBaseUrls((current) => ({ ...current, [providerId]: baseUrl }));
    updateAiProviderConfig({
      id: provider.id,
      endpointMode,
      baseUrl,
      enabled: provider.enabled,
    });
  };

  const handleProviderBaseUrlReset = (providerId: string, providerKind: ProviderKind) => {
    const provider = settings?.aiProviderConfigs.find((item) => item.id === providerId);
    if (!provider) {
      return;
    }
    const baseUrl = PROVIDER_DEFAULT_ENDPOINTS[providerKind];
    setProviderBaseUrls((current) => ({ ...current, [providerId]: baseUrl }));
    updateAiProviderConfig({
      id: provider.id,
      endpointMode: "public",
      baseUrl,
      enabled: provider.enabled,
    });
  };

  const handleAddModelProfile = async () => {
    if (!newModelProviderId || !newModelDisplayName.trim() || !newModelId.trim()) {
      return;
    }
    const success = await addAiModelProfile({
      providerConfigId: newModelProviderId,
      displayName: newModelDisplayName.trim(),
      modelId: newModelId.trim(),
    });
    if (success) {
      setNewModelDisplayName("");
      setNewModelId("");
    }
  };

  const beginEditModelProfile = (model: AiModelProfile) => {
    setEditingModelId(model.id);
    setEditModelProviderId(model.providerConfigId);
    setEditModelDisplayName(model.displayName);
    setEditModelId(model.modelId);
  };

  const cancelEditModelProfile = () => {
    setEditingModelId(null);
    setEditModelProviderId("");
    setEditModelDisplayName("");
    setEditModelId("");
  };

  const handleUpdateModelProfile = async (model: AiModelProfile) => {
    if (!editModelProviderId || !editModelDisplayName.trim() || !editModelId.trim()) {
      return;
    }
    const success = await updateAiModelProfile({
      id: model.id,
      providerConfigId: editModelProviderId,
      displayName: editModelDisplayName.trim(),
      modelId: editModelId.trim(),
      supportsStreaming: model.supportsStreaming,
      maxOutputTokens: model.maxOutputTokens,
    });
    if (success) {
      cancelEditModelProfile();
    }
  };

  const handleMaxHistoryChange = (value: number) => {
    updateSettings({ maxHistoryCount: value });
  };

  const handleDoublePressIntervalChange = (value: number) => {
    updateSettings({ doublePressInterval: value });
  };

  const handleLaunchAtLoginChange = (enabled: boolean) => {
    updateSettings({ launchAtLogin: enabled });
  };

  const handlePasteDelayChange = (value: number) => {
    updateSettings({ pasteDelayMs: value });
  };

  const handleRequestAccessibility = async () => {
    try {
      await invoke("request_accessibility_permission");
      if (accessibilityCheckTimeoutRef.current) {
        clearTimeout(accessibilityCheckTimeoutRef.current);
      }
      accessibilityCheckTimeoutRef.current = setTimeout(() => {
        void (async () => {
          if (!isMountedRef.current) {
            return;
          }
          const granted = await checkAccessibilityStatus();
          if (!granted || !isMountedRef.current) {
            return;
          }
          try {
            await invoke<boolean>("start_hotkey_monitor");
          } catch (err) {
            console.error("Failed to start hotkey monitor after permission grant:", err);
          }
        })();
      }, 1000);
    } catch (err) {
      console.error("Failed to request accessibility:", err);
    }
  };

  const handleCheckForUpdate = async () => {
    clearUpdateError();
    await checkForUpdate(true);
  };

  const handleInstallUpdate = async () => {
    clearUpdateError();
    const installed = await installUpdate();

    if (!installed) {
      return;
    }

    try {
      await relaunch();
    } catch (err) {
      console.error("Failed to relaunch after update:", err);
    }
  };

  if (!settings) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
      </div>
    );
  }

  const preferredModelId = settings.preferredModelProfileId ?? DEFAULT_MODEL_PROFILE_ID;
  const preferredModel = settings.aiModelProfiles.find((model) => model.id === preferredModelId);
  const preferredProvider = preferredModel
    ? settings.aiProviderConfigs.find((provider) => provider.id === preferredModel.providerConfigId)
    : undefined;
  const activeGatewayProviders = settings.aiProviderConfigs.filter(
    (provider) => provider.endpointMode === "custom"
  );
  const customKeyReady = aiApiKeyStatus[CUSTOM_ENDPOINT_API_KEY_ACCOUNT]?.exists ?? false;
  let preferredConnectionLabel = "선택 필요";
  if (preferredProvider) {
    preferredConnectionLabel = preferredProvider.endpointMode === "custom" ? "Gateway" : "Public API";
  }

  return (
    <div className="h-full overflow-y-auto p-4">
      {/* 2열 그리드 레이아웃 */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <section className="md:col-span-2 overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm">
          <div className="flex flex-wrap items-start justify-between gap-3 border-b border-slate-200 px-4 py-3">
            <div>
              <h2 className="text-sm font-semibold text-slate-900">AI 설정</h2>
              <p className="mt-0.5 text-xs text-slate-500">
                기본 모델만 고르면 바로 사용할 수 있습니다.
              </p>
            </div>
            <button
              type="button"
              onClick={() => setIsAiAdvancedOpen((open) => !open)}
              className="rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-medium text-slate-700 transition-colors hover:bg-slate-50"
            >
              {isAiAdvancedOpen ? "고급 설정 닫기" : "고급 설정"}
            </button>
          </div>

          <div className="grid grid-cols-1 gap-0 md:grid-cols-[minmax(320px,1fr)_minmax(360px,1fr)]">
            <div className="space-y-3 border-b border-slate-200 px-4 py-4 md:border-b-0 md:border-r">
              <div className="space-y-1.5">
                <label className="block text-xs font-medium text-slate-600">
                  기본 모델
                </label>
                <select
                  value={preferredModelId}
                  onChange={(e) => handleModelChange(e.target.value)}
                  className="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
                  disabled={isLoading}
                >
                  {settings.aiModelProfiles.map((model) => (
                    <option key={model.id} value={model.id}>
                      {formatModelProfileOption(
                        model,
                        settings.aiProviderConfigs,
                        preferredModelId
                      )}
                    </option>
                  ))}
                </select>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs">
                <div className="rounded-md bg-slate-50 px-3 py-2">
                  <div className="text-slate-500">Provider</div>
                  <div className="mt-1 truncate font-medium text-slate-900">
                    {preferredProvider
                      ? providerLabel(preferredProvider.providerKind)
                      : "선택 필요"}
                  </div>
                </div>
                <div className="rounded-md bg-slate-50 px-3 py-2">
                  <div className="text-slate-500">연결</div>
                  <div className="mt-1 font-medium text-slate-900">
                    {preferredConnectionLabel}
                  </div>
                </div>
              </div>
            </div>

            <div className="px-4 py-4">
              <div className="mb-2 flex items-center justify-between">
                <h3 className="text-xs font-semibold text-slate-700">Provider 상태</h3>
                {activeGatewayProviders.length > 0 && (
                  <span className="rounded-full bg-blue-50 px-2 py-0.5 text-[10px] font-medium text-blue-700">
                    Gateway {activeGatewayProviders.length}
                  </span>
                )}
              </div>
              <div className="divide-y divide-slate-100 rounded-md border border-slate-200">
                {settings.aiProviderConfigs.map((provider) => {
                  const account = providerKeyAccount(provider);
                  const hasKey = aiApiKeyStatus[account]?.exists ?? false;
                  return (
                    <div key={provider.id} className="flex items-center gap-3 px-3 py-2 text-xs">
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-medium text-slate-800">
                          {providerLabel(provider.providerKind)}
                        </div>
                        <div className="truncate text-[11px] text-slate-500">
                          {provider.endpointMode === "custom" ? provider.baseUrl : "Public endpoint"}
                        </div>
                      </div>
                      <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                        provider.endpointMode === "custom"
                          ? "bg-blue-50 text-blue-700"
                          : "bg-slate-100 text-slate-600"
                      }`}>
                        {provider.endpointMode === "custom" ? "Gateway" : "Public"}
                      </span>
                      <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                        hasKey ? "bg-emerald-50 text-emerald-700" : "bg-amber-50 text-amber-700"
                      }`}>
                        {hasKey ? "Key 저장됨" : "Key 필요"}
                      </span>
                    </div>
                  );
                })}
              </div>
              {activeGatewayProviders.length > 0 && (
                <p className={`mt-2 text-[11px] ${
                  customKeyReady ? "text-slate-500" : "text-amber-700"
                }`}>
                  {customKeyReady
                    ? "Gateway 공통 API 키가 설정되어 있습니다."
                    : "Gateway 사용 시 공통 API 키를 저장해야 합니다."}
                </p>
              )}
              {error && (
                <p className="mt-2 text-[11px] text-red-600">{error}</p>
              )}
            </div>
          </div>

          {isAiAdvancedOpen && (
            <div className="border-t border-slate-200 bg-slate-50/60 px-4 py-4">
              <div className="grid grid-cols-1 gap-5 lg:grid-cols-[minmax(0,1.1fr)_minmax(320px,0.9fr)]">
                <div>
                  <div className="mb-2 flex items-center justify-between">
                    <h3 className="text-xs font-semibold text-slate-700">Provider 연결</h3>
                    <span className="text-[10px] text-slate-500">URL은 Gateway 모드에서만 수정됩니다.</span>
                  </div>
                  <div className="space-y-2">
                    {settings.aiProviderConfigs.map((provider) => {
                      const publicAccount = `provider:${provider.id}`;
                      return (
                        <div key={provider.id} className="rounded-md border border-slate-200 bg-white p-3">
                          <div className="flex flex-wrap items-center justify-between gap-2">
                            <div>
                              <div className="text-xs font-medium text-slate-800">
                                {providerLabel(provider.providerKind)}
                              </div>
                              <div className="text-[11px] text-slate-500">
                                {provider.endpointMode === "custom" ? "Gateway URL 사용" : "Public API 사용"}
                              </div>
                            </div>
                            <div className="inline-flex rounded-md border border-slate-300 bg-slate-100 p-0.5">
                              {(["public", "custom"] as EndpointMode[]).map((mode) => (
                                <button
                                  key={mode}
                                  type="button"
                                  onClick={() => handleProviderEndpointModeChange(provider.id, mode)}
                                  disabled={isLoading}
                                  className={`rounded px-2 py-1 text-[11px] font-medium transition-colors ${
                                    provider.endpointMode === mode
                                      ? "bg-white text-slate-900 shadow-sm"
                                      : "text-slate-500 hover:text-slate-800"
                                  }`}
                                >
                                  {mode === "public" ? "Public" : "Gateway"}
                                </button>
                              ))}
                            </div>
                          </div>
                          <div className="mt-3 flex items-center gap-2">
                            <input
                              type="url"
                              value={providerBaseUrls[provider.id] ?? provider.baseUrl}
                              onChange={(e) => setProviderBaseUrls((current) => ({
                                ...current,
                                [provider.id]: e.target.value,
                              }))}
                              onBlur={() => handleProviderBaseUrlSave(provider.id)}
                              onKeyDown={(e) => {
                                if (e.key === "Enter") {
                                  e.currentTarget.blur();
                                }
                              }}
                              placeholder={PROVIDER_DEFAULT_ENDPOINTS[provider.providerKind]}
                              className="min-w-0 flex-1 rounded-md border border-slate-300 bg-white px-2.5 py-1.5 text-xs text-slate-900 outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-100 disabled:bg-slate-100 disabled:text-slate-500"
                              disabled={isLoading || provider.endpointMode === "public"}
                            />
                            <button
                              type="button"
                              onMouseDown={(e) => e.preventDefault()}
                              onClick={() => handleProviderBaseUrlReset(provider.id, provider.providerKind)}
                              disabled={isLoading || provider.endpointMode === "public"}
                              className="shrink-0 rounded-md border border-slate-300 px-2 py-1.5 text-[11px] font-medium text-slate-600 transition-colors hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
                            >
                              기본값
                            </button>
                          </div>
                          {provider.endpointMode === "public" && (
                            <div className="mt-3 border-t border-slate-100 pt-3">
                              <ApiKeyInput
                                account={publicAccount}
                                hasApiKey={aiApiKeyStatus[publicAccount]?.exists ?? false}
                                label={`${providerLabel(provider.providerKind)} API 키`}
                                description="Public API 호출에 사용됩니다."
                              />
                            </div>
                          )}
                        </div>
                      );
                    })}
                    {activeGatewayProviders.length > 0 && (
                      <div className="rounded-md border border-slate-200 bg-white p-3">
                        <ApiKeyInput
                          account={CUSTOM_ENDPOINT_API_KEY_ACCOUNT}
                          hasApiKey={customKeyReady}
                          label="Gateway 공통 API 키"
                          description="Gateway 모드의 provider가 함께 사용합니다."
                        />
                      </div>
                    )}
                  </div>
                </div>

                <div>
                  <h3 className="mb-2 text-xs font-semibold text-slate-700">모델 관리</h3>
                  <div className="space-y-2 rounded-md border border-slate-200 bg-white p-3">
                    <select
                      value={newModelProviderId}
                      onChange={(e) => setNewModelProviderId(e.target.value)}
                      className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs text-slate-900"
                      disabled={isLoading}
                    >
                      {settings.aiProviderConfigs.map((provider) => (
                        <option key={provider.id} value={provider.id}>
                          {providerLabel(provider.providerKind)}
                        </option>
                      ))}
                    </select>
                    <input
                      value={newModelDisplayName}
                      onChange={(e) => setNewModelDisplayName(e.target.value)}
                      placeholder="표시 이름"
                      className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs text-slate-900"
                      disabled={isLoading}
                    />
                    <div className="flex items-center gap-2">
                      <input
                        value={newModelId}
                        onChange={(e) => setNewModelId(e.target.value)}
                        placeholder="model id"
                        className="min-w-0 flex-1 rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs text-slate-900"
                        disabled={isLoading}
                      />
                      <button
                        type="button"
                        onClick={handleAddModelProfile}
                        disabled={isLoading || !newModelDisplayName.trim() || !newModelId.trim()}
                        className="shrink-0 rounded-md bg-blue-600 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
                      >
                        추가
                      </button>
                    </div>
                  </div>

                  <div className="mt-3 max-h-64 overflow-y-auto rounded-md border border-slate-200 bg-white">
                    {settings.aiModelProfiles.map((model) => {
                      const isPreferred = model.id === preferredModelId;
                      const isEditing = editingModelId === model.id;
                      return (
                        <div key={model.id} className="border-b border-slate-100 p-3 text-xs last:border-b-0">
                          {isEditing ? (
                            <div className="space-y-2">
                              <select
                                value={editModelProviderId}
                                onChange={(e) => setEditModelProviderId(e.target.value)}
                                className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs"
                                disabled={isLoading}
                              >
                                {settings.aiProviderConfigs.map((provider) => (
                                  <option key={provider.id} value={provider.id}>
                                    {providerLabel(provider.providerKind)}
                                  </option>
                                ))}
                              </select>
                              <input
                                value={editModelDisplayName}
                                onChange={(e) => setEditModelDisplayName(e.target.value)}
                                className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs"
                                disabled={isLoading}
                              />
                              <input
                                value={editModelId}
                                onChange={(e) => setEditModelId(e.target.value)}
                                className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 text-xs"
                                disabled={isLoading}
                              />
                              <div className="flex justify-end gap-1">
                                <button
                                  type="button"
                                  onClick={() => handleUpdateModelProfile(model)}
                                  className="rounded bg-blue-600 px-2 py-1 text-[11px] font-medium text-white transition-colors hover:bg-blue-700 disabled:opacity-50"
                                  disabled={isLoading || !editModelDisplayName.trim() || !editModelId.trim()}
                                >
                                  저장
                                </button>
                                <button
                                  type="button"
                                  onClick={cancelEditModelProfile}
                                  className="rounded px-2 py-1 text-[11px] text-slate-600 transition-colors hover:bg-slate-100"
                                  disabled={isLoading}
                                >
                                  취소
                                </button>
                              </div>
                            </div>
                          ) : (
                            <div className="flex items-center justify-between gap-2">
                              <div className="min-w-0">
                                <div className="truncate font-medium text-slate-800">
                                  {formatModelProfileOption(model, settings.aiProviderConfigs, preferredModelId)}
                                </div>
                                <div className="truncate text-[11px] text-slate-500">
                                  {model.modelId}
                                </div>
                              </div>
                              <div className="shrink-0 flex items-center gap-1">
                                <button
                                  type="button"
                                  onClick={() => beginEditModelProfile(model)}
                                  className="rounded px-1.5 py-0.5 text-[11px] text-slate-600 transition-colors hover:bg-slate-100"
                                  disabled={isLoading}
                                >
                                  수정
                                </button>
                                {!isPreferred && (
                                  <button
                                    type="button"
                                    onClick={() => deleteAiModelProfile(model.id)}
                                    className="rounded px-1.5 py-0.5 text-[11px] text-red-600 transition-colors hover:bg-red-50"
                                    disabled={isLoading}
                                  >
                                    삭제
                                  </button>
                                )}
                              </div>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              </div>
            </div>
          )}
        </section>

        <section className="md:col-span-2 overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm">
          <div className="border-b border-slate-200 px-4 py-3">
            <h2 className="text-sm font-semibold text-slate-900">사용 환경</h2>
            <p className="mt-0.5 text-xs text-slate-500">
              단축키, 저장 개수, 붙여넣기 동작을 조정합니다.
            </p>
          </div>

          <div className="grid grid-cols-1 divide-y divide-slate-200 md:grid-cols-2 md:divide-x md:divide-y-0">
            <div className="divide-y divide-slate-100">
              <RangeSetting
                title="단축키 감지"
                valueLabel={`${settings.doublePressInterval}ms`}
                description="더블 프레스 입력으로 번역을 시작하는 시간 간격입니다."
                min={200}
                max={1000}
                step={50}
                value={settings.doublePressInterval}
                onChange={handleDoublePressIntervalChange}
                disabled={isLoading}
              />
              <RangeSetting
                title="클립보드 저장"
                valueLabel={`${settings.maxHistoryCount}개`}
                description="최근 복사 기록을 보관할 최대 개수입니다."
                min={10}
                max={200}
                step={10}
                value={settings.maxHistoryCount}
                onChange={handleMaxHistoryChange}
                disabled={isLoading}
              />
            </div>

            <div className="divide-y divide-slate-100">
              <RangeSetting
                title="붙여넣기 딜레이"
                valueLabel={`${settings.pasteDelayMs}ms`}
                description="붙여넣기 실패가 있을 때만 값을 조금 높여보세요."
                min={50}
                max={500}
                step={25}
                value={settings.pasteDelayMs}
                onChange={handlePasteDelayChange}
                disabled={isLoading}
              />

              <div className="px-4 py-4">
                <div className="flex items-center justify-between gap-4">
                  <div>
                    <div className="text-xs font-semibold text-slate-900">로그인 시 시작</div>
                    <p className="mt-1 text-[11px] text-slate-500">
                      macOS 로그인 후 TransClip을 자동으로 실행합니다.
                    </p>
                  </div>
                  <ToggleSwitch
                    enabled={settings.launchAtLogin}
                    onChange={handleLaunchAtLoginChange}
                    disabled={isLoading}
                    label="로그인 시 시작"
                  />
                </div>

                <div className="mt-4 flex items-center justify-between gap-4 rounded-md bg-slate-50 px-3 py-2">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-medium text-slate-800">접근성 권한</span>
                      {accessibilityGranted === true && (
                        <StatusBadge tone="success">허용됨</StatusBadge>
                      )}
                      {accessibilityGranted === false && (
                        <StatusBadge tone="warning">필요</StatusBadge>
                      )}
                    </div>
                    <p className="mt-0.5 truncate text-[11px] text-slate-500">
                      더블 프레스 감지에 필요합니다.
                    </p>
                  </div>
                  {accessibilityGranted === false && (
                    <button
                      onClick={handleRequestAccessibility}
                      className="shrink-0 rounded-md bg-blue-600 px-2.5 py-1.5 text-[11px] font-medium text-white transition-colors hover:bg-blue-700"
                    >
                      허용
                    </button>
                  )}
                  {accessibilityGranted === true && (
                    <button
                      onClick={checkAccessibilityStatus}
                      className="shrink-0 rounded-md border border-slate-300 px-2.5 py-1.5 text-[11px] font-medium text-slate-600 transition-colors hover:bg-white"
                    >
                      새로고침
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>

      <div className="mt-5 space-y-2 border-t border-slate-200 pt-4 text-xs text-slate-500">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <span>TransClip v{currentVersion ?? "..."}</span>
            {hasUpdate && latestVersion && (
              <span className="rounded-full bg-emerald-50 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700">
                새 버전 v{latestVersion}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              className="rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-medium text-slate-600 transition-colors hover:bg-slate-50 disabled:opacity-50"
              onClick={handleCheckForUpdate}
              disabled={isCheckingUpdate || isDownloading}
            >
              {isCheckingUpdate ? "확인 중..." : "업데이트 확인"}
            </button>

            {hasUpdate && (
              <>
                <button
                  type="button"
                  className="rounded-md bg-blue-600 px-2.5 py-1.5 text-xs font-medium text-white transition-colors hover:bg-blue-700 disabled:opacity-50"
                  onClick={handleInstallUpdate}
                  disabled={isDownloading}
                >
                  {isDownloading ? `다운로드 중... ${progress}%` : "업데이트"}
                </button>
                <button
                  type="button"
                  className="rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-medium text-slate-600 transition-colors hover:bg-slate-50 disabled:opacity-50"
                  onClick={dismissUpdate}
                  disabled={isDownloading}
                >
                  나중에
                </button>
              </>
            )}

            <button
              type="button"
              className="flex items-center gap-1 rounded-md px-1.5 py-1 text-slate-500 transition-colors hover:bg-slate-50 hover:text-blue-700"
              onClick={() => {
                void invoke("open_feedback_page").catch((err) => {
                  console.error("Failed to open feedback page:", err);
                });
              }}
            >
              <span>버그 제보 · 기능 제안</span>
              <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
              </svg>
            </button>
          </div>
        </div>

        {isDownloading && (
          <div className="space-y-1">
            <div
              className="h-1.5 overflow-hidden rounded-full bg-slate-200"
              role="progressbar"
              aria-label="업데이트 다운로드 진행률"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progress}
              aria-valuetext={`${progress}%`}
            >
              <div className="h-full bg-blue-600 transition-all" style={{ width: `${progress}%` }} />
            </div>
            <p className="text-[10px] text-slate-500">업데이트 다운로드 중 ({progress}%)</p>
          </div>
        )}

        {updateError && (
          <p className="text-[10px] text-red-600">
            업데이트 오류: {updateError}
          </p>
        )}
      </div>
    </div>
  );
}

interface RangeSettingProps {
  title: string;
  valueLabel: string;
  description: string;
  min: number;
  max: number;
  step: number;
  value: number;
  onChange: (value: number) => void;
  disabled?: boolean;
}

function RangeSetting({
  title,
  valueLabel,
  description,
  min,
  max,
  step,
  value,
  onChange,
  disabled,
}: RangeSettingProps) {
  return (
    <div className="px-4 py-4">
      <div className="mb-3 flex items-start justify-between gap-4">
        <div>
          <div className="text-xs font-semibold text-slate-900">{title}</div>
          <p className="mt-1 text-[11px] text-slate-500">{description}</p>
        </div>
        <span className="shrink-0 rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700">
          {valueLabel}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-blue-600"
        disabled={disabled}
      />
    </div>
  );
}

interface StatusBadgeProps {
  tone: "success" | "warning";
  children: React.ReactNode;
}

function StatusBadge({ tone, children }: StatusBadgeProps) {
  const className = tone === "success"
    ? "bg-emerald-50 text-emerald-700"
    : "bg-amber-50 text-amber-700";
  return (
    <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${className}`}>
      {children}
    </span>
  );
}

interface ToggleSwitchProps {
  enabled: boolean;
  onChange: (enabled: boolean) => void;
  disabled?: boolean;
  label: string;
}

function ToggleSwitch({ enabled, onChange, disabled, label }: ToggleSwitchProps) {
  return (
    <button
      onClick={() => onChange(!enabled)}
      aria-pressed={enabled}
      aria-label={`${label} ${enabled ? "켜짐" : "꺼짐"}`}
      aria-disabled={disabled}
      className={`relative h-5 w-9 rounded-full transition-colors ${
        enabled ? "bg-blue-600" : "bg-slate-300"
      }`}
      disabled={disabled}
    >
      <span
        className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform ${
          enabled ? "translate-x-4" : ""
        }`}
      />
    </button>
  );
}
