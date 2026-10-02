import { useEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { useSettingsStore, useUpdateStore } from "@/store";
import { ApiKeyInput } from "./ApiKeyInput";
import { Badge, Button, Icon, Segmented, Select, Slider, Spinner, Switch } from "@/components/common";
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
      <div className="flex h-full items-center justify-center text-label-3">
        <Spinner size={20} />
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
    <div className="scroll-fade-y h-full overflow-y-auto px-5 pb-6 pt-4">
      <div className="mx-auto grid max-w-[1120px] grid-cols-1 items-start gap-x-6 gap-y-5 md:grid-cols-2">
        {/* Column 1 — AI */}
        <div className="space-y-5">
          <SettingsSection title="AI 모델">
            <SettingsRow title="기본 모델" description="번역과 글 다듬기에 사용합니다.">
              <Select
                aria-label="기본 모델"
                block
                wrapperClassName="w-[270px]"
                value={preferredModelId}
                onChange={(e) => handleModelChange(e.target.value)}
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
              </Select>
            </SettingsRow>

            <SettingsRow title="Provider">
              <span className="text-body text-label-2">
                {preferredProvider ? providerLabel(preferredProvider.providerKind) : "선택 필요"}
              </span>
            </SettingsRow>

            <SettingsRow title="연결">
              <Badge tone={preferredProvider?.endpointMode === "custom" ? "accent" : undefined}>
                {preferredConnectionLabel}
              </Badge>
            </SettingsRow>

            <button
              type="button"
              onClick={() => setIsAiAdvancedOpen((open) => !open)}
              aria-expanded={isAiAdvancedOpen}
              className="form-row w-full text-left transition-colors hover:bg-fill"
            >
              <div className="min-w-0 flex-1">
                <div className="text-body text-label">고급 설정</div>
                <p className="mt-0.5 text-caption text-label-3">
                  Provider 연결, API 키, 모델 목록을 직접 관리합니다.
                </p>
              </div>
              <Icon
                name="chevron-down"
                size={15}
                strokeWidth={2}
                className={`shrink-0 text-label-3 transition-transform duration-200 ${
                  isAiAdvancedOpen ? "rotate-180" : ""
                }`}
              />
            </button>
          </SettingsSection>

          <SettingsSection
            title="Provider 상태"
            footer={
              <>
                {activeGatewayProviders.length > 0 && (
                  <span className={customKeyReady ? "" : "text-orange-fg"}>
                    {customKeyReady
                      ? "Gateway 공통 API 키가 설정되어 있습니다."
                      : "Gateway 사용 시 공통 API 키를 저장해야 합니다."}
                  </span>
                )}
                {error && <span className="block text-red-fg">{error}</span>}
              </>
            }
          >
            {settings.aiProviderConfigs.map((provider) => {
              const account = providerKeyAccount(provider);
              const hasKey = aiApiKeyStatus[account]?.exists ?? false;
              return (
                <SettingsRow
                  key={provider.id}
                  title={providerLabel(provider.providerKind)}
                  description={provider.endpointMode === "custom" ? provider.baseUrl : "Public endpoint"}
                >
                  <Badge tone={provider.endpointMode === "custom" ? "accent" : undefined}>
                    {provider.endpointMode === "custom" ? "Gateway" : "Public"}
                  </Badge>
                  <Badge tone={hasKey ? "green" : "orange"} icon={hasKey ? "check" : undefined}>
                    {hasKey ? "Key 저장됨" : "Key 필요"}
                  </Badge>
                </SettingsRow>
              );
            })}
          </SettingsSection>
        </div>

        {/* Column 2 — Usage */}
        <div className="space-y-5">
          <SettingsSection title="사용 환경">
            <SliderRow
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
            <SliderRow
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
            <SliderRow
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

            <SettingsRow
              title="로그인 시 시작"
              description="macOS 로그인 후 TransClip을 자동으로 실행합니다."
            >
              <Switch
                checked={settings.launchAtLogin}
                onChange={handleLaunchAtLoginChange}
                disabled={isLoading}
                label="로그인 시 시작"
              />
            </SettingsRow>

            <SettingsRow title="접근성 권한" description="더블 프레스 감지에 필요합니다.">
              {accessibilityGranted === true && (
                <Badge tone="green" icon="check">
                  허용됨
                </Badge>
              )}
              {accessibilityGranted === false && <Badge tone="orange">필요</Badge>}
              {accessibilityGranted === false && (
                <Button size="sm" variant="primary" onClick={handleRequestAccessibility}>
                  허용
                </Button>
              )}
              {accessibilityGranted === true && (
                <Button size="sm" onClick={() => void checkAccessibilityStatus()}>
                  새로고침
                </Button>
              )}
            </SettingsRow>
          </SettingsSection>

          <SettingsSection title="정보">
            <SettingsRow
              title={
                <span className="flex flex-wrap items-center gap-2">
                  <span>TransClip v{currentVersion ?? "..."}</span>
                  {hasUpdate && latestVersion && <Badge tone="green">새 버전 v{latestVersion}</Badge>}
                </span>
              }
            >
              <Button
                size="sm"
                onClick={handleCheckForUpdate}
                disabled={isCheckingUpdate || isDownloading}
              >
                {isCheckingUpdate ? "확인 중..." : "업데이트 확인"}
              </Button>

              {hasUpdate && (
                <>
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={handleInstallUpdate}
                    disabled={isDownloading}
                  >
                    {isDownloading ? `다운로드 중... ${progress}%` : "업데이트"}
                  </Button>
                  <Button size="sm" variant="plain" onClick={dismissUpdate} disabled={isDownloading}>
                    나중에
                  </Button>
                </>
              )}
            </SettingsRow>

            {isDownloading && (
              <div className="form-row flex-col items-stretch gap-1.5">
                <div
                  className="h-1.5 overflow-hidden rounded-full bg-fill-2"
                  role="progressbar"
                  aria-label="업데이트 다운로드 진행률"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={progress}
                  aria-valuetext={`${progress}%`}
                >
                  <div
                    className="h-full rounded-full bg-accent transition-all"
                    style={{ width: `${progress}%` }}
                  />
                </div>
                <p className="text-caption text-label-3">업데이트 다운로드 중 ({progress}%)</p>
              </div>
            )}

            {updateError && (
              <div className="form-row">
                <p className="text-caption text-red-fg">업데이트 오류: {updateError}</p>
              </div>
            )}

            <button
              type="button"
              className="form-row w-full text-left transition-colors hover:bg-fill"
              onClick={() => {
                void invoke("open_feedback_page").catch((err) => {
                  console.error("Failed to open feedback page:", err);
                });
              }}
            >
              <span className="min-w-0 flex-1 text-body text-label">버그 제보 · 기능 제안</span>
              <Icon name="external-link" size={15} className="shrink-0 text-label-3" />
            </button>
          </SettingsSection>
        </div>

        {/* Advanced AI settings */}
        {isAiAdvancedOpen && (
          <div className="animate-fade-in grid grid-cols-1 items-start gap-x-6 gap-y-5 md:col-span-2 md:grid-cols-2">
            <div className="space-y-3">
              <div className="flex items-baseline justify-between gap-3 px-1">
                <h2 className="text-sub font-semibold text-label-2">Provider 연결</h2>
                <span className="text-caption text-label-3">URL은 Gateway 모드에서만 수정됩니다.</span>
              </div>

              {settings.aiProviderConfigs.map((provider) => {
                const publicAccount = `provider:${provider.id}`;
                return (
                  <div key={provider.id} className="form-group">
                    <SettingsRow
                      title={providerLabel(provider.providerKind)}
                      description={provider.endpointMode === "custom" ? "Gateway URL 사용" : "Public API 사용"}
                    >
                      <Segmented<EndpointMode>
                        ariaLabel={`${providerLabel(provider.providerKind)} 연결 방식`}
                        value={provider.endpointMode}
                        disabled={isLoading}
                        options={[
                          { value: "public", label: "Public" },
                          { value: "custom", label: "Gateway" },
                        ]}
                        onChange={(mode) => handleProviderEndpointModeChange(provider.id, mode)}
                      />
                    </SettingsRow>

                    <div className="form-row">
                      <input
                        type="url"
                        aria-label={`${providerLabel(provider.providerKind)} 엔드포인트 URL`}
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
                        className="field field-sm min-w-0 flex-1"
                        disabled={isLoading || provider.endpointMode === "public"}
                      />
                      <Button
                        size="sm"
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() => handleProviderBaseUrlReset(provider.id, provider.providerKind)}
                        disabled={isLoading || provider.endpointMode === "public"}
                      >
                        기본값
                      </Button>
                    </div>

                    {provider.endpointMode === "public" && (
                      <div className="form-row">
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
                <div className="form-group">
                  <div className="form-row">
                    <ApiKeyInput
                      account={CUSTOM_ENDPOINT_API_KEY_ACCOUNT}
                      hasApiKey={customKeyReady}
                      label="Gateway 공통 API 키"
                      description="Gateway 모드의 provider가 함께 사용합니다."
                    />
                  </div>
                </div>
              )}
            </div>

            <div className="space-y-3">
              <h2 className="px-1 text-sub font-semibold text-label-2">모델 관리</h2>

              <div className="form-group">
                <div className="form-row">
                  <Select
                    aria-label="새 모델의 Provider"
                    block
                    value={newModelProviderId}
                    onChange={(e) => setNewModelProviderId(e.target.value)}
                    disabled={isLoading}
                  >
                    {settings.aiProviderConfigs.map((provider) => (
                      <option key={provider.id} value={provider.id}>
                        {providerLabel(provider.providerKind)}
                      </option>
                    ))}
                  </Select>
                </div>
                <div className="form-row">
                  <input
                    value={newModelDisplayName}
                    onChange={(e) => setNewModelDisplayName(e.target.value)}
                    placeholder="표시 이름"
                    aria-label="표시 이름"
                    className="field field-sm"
                    disabled={isLoading}
                  />
                </div>
                <div className="form-row">
                  <input
                    value={newModelId}
                    onChange={(e) => setNewModelId(e.target.value)}
                    placeholder="model id"
                    aria-label="model id"
                    className="field field-sm min-w-0 flex-1"
                    disabled={isLoading}
                  />
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={handleAddModelProfile}
                    disabled={isLoading || !newModelDisplayName.trim() || !newModelId.trim()}
                  >
                    추가
                  </Button>
                </div>
              </div>

              <div className="form-group max-h-64 overflow-y-auto">
                {settings.aiModelProfiles.map((model) => {
                  const isPreferred = model.id === preferredModelId;
                  const isEditing = editingModelId === model.id;
                  return isEditing ? (
                    <div key={model.id} className="form-row flex-col items-stretch gap-2">
                      <Select
                        aria-label="Provider"
                        block
                        value={editModelProviderId}
                        onChange={(e) => setEditModelProviderId(e.target.value)}
                        disabled={isLoading}
                      >
                        {settings.aiProviderConfigs.map((provider) => (
                          <option key={provider.id} value={provider.id}>
                            {providerLabel(provider.providerKind)}
                          </option>
                        ))}
                      </Select>
                      <input
                        value={editModelDisplayName}
                        onChange={(e) => setEditModelDisplayName(e.target.value)}
                        aria-label="표시 이름"
                        className="field field-sm"
                        disabled={isLoading}
                      />
                      <input
                        value={editModelId}
                        onChange={(e) => setEditModelId(e.target.value)}
                        aria-label="model id"
                        className="field field-sm"
                        disabled={isLoading}
                      />
                      <div className="flex justify-end gap-1.5">
                        <Button size="sm" variant="plain" onClick={cancelEditModelProfile} disabled={isLoading}>
                          취소
                        </Button>
                        <Button
                          size="sm"
                          variant="primary"
                          onClick={() => handleUpdateModelProfile(model)}
                          disabled={isLoading || !editModelDisplayName.trim() || !editModelId.trim()}
                        >
                          저장
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <SettingsRow
                      key={model.id}
                      title={formatModelProfileOption(model, settings.aiProviderConfigs, preferredModelId)}
                      description={model.modelId}
                    >
                      <Button
                        size="sm"
                        variant="plain"
                        onClick={() => beginEditModelProfile(model)}
                        disabled={isLoading}
                      >
                        수정
                      </Button>
                      {!isPreferred && (
                        <Button
                          size="sm"
                          variant="plain"
                          className="hover:!text-red-fg"
                          onClick={() => deleteAiModelProfile(model.id)}
                          disabled={isLoading}
                        >
                          삭제
                        </Button>
                      )}
                    </SettingsRow>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

interface SettingsSectionProps {
  title: string;
  footer?: ReactNode;
  children: ReactNode;
}

/** Titled, inset group of rows — the System Settings pattern. */
function SettingsSection({ title, footer, children }: SettingsSectionProps) {
  return (
    <section>
      <h2 className="mb-1.5 px-1 text-sub font-semibold text-label-2">{title}</h2>
      <div className="form-group">{children}</div>
      {footer && <p className="mt-1.5 space-y-0.5 px-1 text-caption text-label-3">{footer}</p>}
    </section>
  );
}

interface SettingsRowProps {
  title: ReactNode;
  description?: string;
  children?: ReactNode;
}

function SettingsRow({ title, description, children }: SettingsRowProps) {
  return (
    <div className="form-row">
      <div className="min-w-0 flex-1">
        <div className="text-body text-label">{title}</div>
        {description && <p className="mt-0.5 text-caption text-label-3">{description}</p>}
      </div>
      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}

interface SliderRowProps {
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

function SliderRow({
  title,
  valueLabel,
  description,
  min,
  max,
  step,
  value,
  onChange,
  disabled,
}: SliderRowProps) {
  return (
    <SettingsRow title={title} description={description}>
      <Slider
        label={title}
        className="w-[170px]"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={onChange}
        disabled={disabled}
      />
      <span className="w-[52px] text-right text-sub tabular-nums text-label-2">{valueLabel}</span>
    </SettingsRow>
  );
}
