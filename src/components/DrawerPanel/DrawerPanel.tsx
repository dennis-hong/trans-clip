import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Event as TauriEvent } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/store";
import { useWindowDrag } from "@/hooks/useWindowDrag";
import { PostItCard, POSTIT_CARD_WIDTH } from "./PostItCard";
import { CreatePostItCard } from "./CreatePostItCard";
import { ShortcutsPopover } from "./ShortcutsPopover";
import {
  Button,
  Icon,
  IconButton,
  Kbd,
  Segmented,
  Spinner,
  Toast,
} from "@/components/common";
import { SettingsPanel } from "@/components/Settings/SettingsPanel";
import { GlossaryList } from "@/components/GlossaryManager/GlossaryList";
import type { ClipboardItem, ClipboardChangedPayload } from "@/types";

interface MonitorInfo {
  name: string | null;
  positionX: number;
  positionY: number;
  width: number;
  height: number;
  scaleFactor: number;
  isPrimary: boolean;
}

type DrawerView = "history" | "settings" | "glossary";
type DrawerMode = "collapsed" | "expanded" | "full";

/** Gap between notes in the history strip (px). */
const CARD_GAP = 12;

function ShortcutHint({ keys, label, title }: { keys: string; label: string; title: string }) {
  return (
    <span className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap" title={title}>
      <Kbd>{keys}</Kbd>
      {/* Labels give way to the keycaps when the window is narrow. */}
      <span className="hidden lg:inline">{label}</span>
    </span>
  );
}

interface DrawerPanelProps {
  hasAccessibility?: boolean | null;
  onClose?: () => void;
  isStealthMode?: boolean;
  onTranslate?: (text: string) => void;
  onPolish?: (text: string) => void;
  openSettingsSignal?: number;
  savedMonitorIndex?: number | null;
  onMonitorChange?: (index: number) => void;
}

export function DrawerPanel({
  hasAccessibility,
  onClose,
  isStealthMode,
  onTranslate,
  onPolish,
  openSettingsSignal,
  savedMonitorIndex,
  onMonitorChange,
}: DrawerPanelProps) {
  const [currentView, setCurrentView] = useState<DrawerView>("history");
  const [drawerMode, setDrawerMode] = useState<DrawerMode>("expanded");
  const [searchQuery, setSearchQuery] = useState("");
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [toolbarSlot, setToolbarSlot] = useState<HTMLDivElement | null>(null);
  const [currentMonitorInternal, setCurrentMonitorInternal] = useState(savedMonitorIndex ?? 0);
  const setCurrentMonitor = useCallback((index: number) => {
    setCurrentMonitorInternal(index);
    onMonitorChange?.(index);
  }, [onMonitorChange]);
  const currentMonitor = currentMonitorInternal;
  const { handleDragStart } = useWindowDrag({
    onDragEnd: async () => {
      try {
        const currentIdx = await invoke<number>("get_current_monitor_index");
        setCurrentMonitor(currentIdx);
        const win = getCurrentWindow();
        const size = await win.outerSize();
        const scaleFactor = await win.scaleFactor();
        lastSavedWidthRef.current = Math.round(size.width / scaleFactor);
      } catch (err) {
        console.error("Failed to update monitor after drag:", err);
      }
    },
  });
  const scrollRef = useRef<HTMLDivElement>(null);
  const lastSavedWidthRef = useRef<number>(0);
  const resizeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingTimeoutsRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  const scheduleTimeout = useCallback((callback: () => void | Promise<void>, delayMs: number) => {
    const timeoutId = setTimeout(() => {
      pendingTimeoutsRef.current.delete(timeoutId);
      void callback();
    }, delayMs);
    pendingTimeoutsRef.current.add(timeoutId);
    return timeoutId;
  }, []);

  const clearPendingTimeouts = useCallback(() => {
    pendingTimeoutsRef.current.forEach((timeoutId) => clearTimeout(timeoutId));
    pendingTimeoutsRef.current.clear();
  }, []);

  const { items, isLoading, fetchHistory, deleteItem, togglePin } = useClipboardStore();

  const loadMonitors = useCallback(async () => {
    try {
      const result = await invoke<MonitorInfo[]>("get_monitors");
      setMonitors(result);

      if (savedMonitorIndex != null && savedMonitorIndex < result.length) {
        setCurrentMonitor(savedMonitorIndex);
      } else {
        const currentIdx = await invoke<number>("get_current_monitor_index");
        setCurrentMonitor(currentIdx);
      }
    } catch (err) {
      console.error("Failed to get monitors:", err);
    }
  }, [savedMonitorIndex, setCurrentMonitor]);

  // Keep a ref to items for keyboard handler (avoids re-registering listener on every items change)
  const itemsRef = useRef(items);
  itemsRef.current = items;

  // Fetch history on mount
  useEffect(() => {
    fetchHistory();
    loadMonitors();

    // Initialize lastSavedWidth with current window width
    const initWidth = async () => {
      try {
        const window = getCurrentWindow();
        const size = await window.outerSize();
        const scaleFactor = await window.scaleFactor();
        lastSavedWidthRef.current = Math.round(size.width / scaleFactor);
      } catch (err) {
        console.error("Failed to get initial window size:", err);
      }
    };
    initWidth();
  }, [fetchHistory, loadMonitors]);

  useEffect(() => {
    return () => {
      clearPendingTimeouts();
    };
  }, [clearPendingTimeouts]);

  // Listen for postit_saved event from editor window
  useEffect(() => {
    const unlisten = listen<{ mode: string; itemId?: string }>("postit_saved", async (event) => {
      // Refresh history when a postit is saved
      await fetchHistory();
      const message = event.payload.mode === "edit" ? "수정되었습니다" : "메모가 생성되었습니다";
      setToast({ message, type: "success" });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchHistory]);

  // Listen for window resize events and save the width when user manually resizes
  useEffect(() => {
    const appWindow = getCurrentWindow();

    const handleResize = async (event: TauriEvent<{ width: number; height: number }>) => {
      // Clear any pending save timeout
      if (resizeTimeoutRef.current) {
        clearTimeout(resizeTimeoutRef.current);
      }

      // Get scale factor to convert to logical width
      const scaleFactor = await appWindow.scaleFactor();
      const logicalWidth = Math.round(event.payload.width / scaleFactor);

      // Only save if width changed significantly (more than 10px) and not from our own programmatic changes
      const widthDiff = Math.abs(logicalWidth - lastSavedWidthRef.current);
      if (widthDiff > 10) {
        // Debounce save to avoid saving during continuous resize
        resizeTimeoutRef.current = setTimeout(async () => {
          try {
            await invoke("save_window_width_for_monitor", { width: logicalWidth });
            lastSavedWidthRef.current = logicalWidth;
          } catch (err) {
            console.error("Failed to save window width:", err);
          }
        }, 500); // Wait 500ms after resize stops
      }
    };

    const unlisten = appWindow.onResized(handleResize);

    return () => {
      if (resizeTimeoutRef.current) {
        clearTimeout(resizeTimeoutRef.current);
      }
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for clipboard changes (debounced to avoid redundant fetches)
  const clipboardDebounceRef = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => {
    const unlisten = listen<ClipboardChangedPayload>("clipboard_changed", () => {
      clearTimeout(clipboardDebounceRef.current);
      clipboardDebounceRef.current = setTimeout(() => fetchHistory(), 100);
    });
    return () => {
      clearTimeout(clipboardDebounceRef.current);
      unlisten.then((fn) => fn());
    };
  }, [fetchHistory]);

  // Handler for creating new post-it (defined before keyboard shortcuts useEffect)
  const handleCreateNewItem = useCallback(async () => {
    try {
      await invoke("open_postit_editor", {
        mode: "create",
      });
    } catch (err) {
      console.error("Failed to open editor:", err);
      setToast({ message: "편집기를 열 수 없습니다", type: "error" });
    }
  }, []);

  // Keyboard shortcuts for monitor switching (Alt+1, Alt+2, Alt+3)
  // and quick selection (number keys 1-9) and ESC to close
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      // ESC key - close the panel (in stealth mode)
      if (e.key === "Escape" && isStealthMode && onClose) {
        e.preventDefault();
        onClose();
        return;
      }

      // Cmd+N - create new post-it (only in history view)
      if (e.key === "n" && e.metaKey && !e.altKey && !e.ctrlKey && !e.shiftKey && currentView === "history") {
        e.preventDefault();
        handleCreateNewItem();
        return;
      }

      // Alt/Option + number keys (1, 2, 3) - monitor switching
      // Use e.code instead of e.key because Option+number produces special characters on macOS
      if (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
        let monitorIndex = -1;

        // Map key codes to monitor indices
        if (e.code === "Digit1" || e.code === "Numpad1") monitorIndex = 0;
        else if (e.code === "Digit2" || e.code === "Numpad2") monitorIndex = 1;
        else if (e.code === "Digit3" || e.code === "Numpad3") monitorIndex = 2;
        else if (e.code === "Digit4" || e.code === "Numpad4") monitorIndex = 3;
        else if (e.code === "Digit5" || e.code === "Numpad5") monitorIndex = 4;

        if (monitorIndex >= 0 && monitorIndex < monitors.length) {
          e.preventDefault();
          try {
            await invoke("move_to_monitor", { monitorIndex, anchor: "bottom" });
            setCurrentMonitor(monitorIndex);

            // Update lastSavedWidthRef with the new window width after monitor change
            scheduleTimeout(async () => {
              try {
                const win = getCurrentWindow();
                const size = await win.outerSize();
                const scaleFactor = await win.scaleFactor();
                lastSavedWidthRef.current = Math.round(size.width / scaleFactor);
              } catch (err) {
                console.error("Failed to update lastSavedWidth:", err);
              }
            }, 100);
          } catch (err) {
            console.error(`Failed to move to monitor ${monitorIndex + 1}:`, err);
          }
        }
        return;
      }

      // History view keyboard shortcuts
      if (currentView === "history") {
        // Arrow keys for scrolling (left/right) - no modifiers
        if (!e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
          if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
            e.preventDefault();
            if (scrollRef.current) {
              const scrollAmount = POSTIT_CARD_WIDTH + CARD_GAP;
              const direction = e.key === "ArrowLeft" ? -1 : 1;
              scrollRef.current.scrollBy({
                left: scrollAmount * direction,
                behavior: "smooth",
              });
            }
            return;
          }
        }

        // Sort items: pinned first, then unpinned (use ref to avoid dep on items array)
        const currentItems = itemsRef.current;
        const pinnedItems = currentItems.filter((item) => item.isPinned);
        const unpinnedItems = currentItems.filter((item) => !item.isPinned);
        const sortedItemsList = [...pinnedItems, ...unpinnedItems];

        // Get item index from number key (use e.code for consistency)
        let itemIndex = -1;
        if (e.code === "Digit1" || e.code === "Numpad1") itemIndex = 0;
        else if (e.code === "Digit2" || e.code === "Numpad2") itemIndex = 1;
        else if (e.code === "Digit3" || e.code === "Numpad3") itemIndex = 2;
        else if (e.code === "Digit4" || e.code === "Numpad4") itemIndex = 3;
        else if (e.code === "Digit5" || e.code === "Numpad5") itemIndex = 4;
        else if (e.code === "Digit6" || e.code === "Numpad6") itemIndex = 5;
        else if (e.code === "Digit7" || e.code === "Numpad7") itemIndex = 6;
        else if (e.code === "Digit8" || e.code === "Numpad8") itemIndex = 7;
        else if (e.code === "Digit9" || e.code === "Numpad9") itemIndex = 8;

        if (itemIndex >= 0 && itemIndex < sortedItemsList.length) {
          const item = sortedItemsList[itemIndex];
          if (!item) return;

          // Shift + number: Translate
          if (e.shiftKey && !e.metaKey && !e.ctrlKey && onTranslate) {
            e.preventDefault();
            onTranslate(item.content);
            return;
          }

          // Ctrl + number: Polish (using Ctrl instead of Alt because Alt+number is for monitor switching)
          if (e.ctrlKey && !e.metaKey && !e.shiftKey && onPolish) {
            e.preventDefault();
            onPolish(item.content);
            return;
          }

          // Number keys without modifiers: quick copy
          if (!e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
            e.preventDefault();
            invoke("set_clipboard", { text: item.content }).then(() => {
              setToast({ message: "클립보드에 복사됨!", type: "success" });
              // In stealth mode, close after copying
              if (isStealthMode && onClose) {
                scheduleTimeout(() => onClose(), 300);
              }
            }).catch((err) => {
              console.error("Failed to copy:", err);
              setToast({ message: "복사 실패", type: "error" });
            });
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [monitors.length, currentView, isStealthMode, onClose, onTranslate, onPolish, handleCreateNewItem, scheduleTimeout, setCurrentMonitor]);

  const updateDrawerMode = useCallback(async (mode: DrawerMode) => {
    setDrawerMode(mode);
    try {
      await invoke("set_drawer_mode", { mode });

      // Update lastSavedWidthRef after mode change (width might have changed)
      scheduleTimeout(async () => {
        try {
          const window = getCurrentWindow();
          const size = await window.outerSize();
          const scaleFactor = await window.scaleFactor();
          lastSavedWidthRef.current = Math.round(size.width / scaleFactor);
        } catch (err) {
          console.error("Failed to update lastSavedWidth:", err);
        }
      }, 100);
    } catch (err) {
      console.error("Failed to set drawer mode:", err);
    }
  }, [scheduleTimeout]);

  const handleSearch = useCallback(
    (query: string) => {
      setSearchQuery(query);
      fetchHistory({ searchQuery: query || undefined });
    },
    [fetchHistory]
  );

  const handleCopy = useCallback(async (item: ClipboardItem) => {
    try {
      await invoke("set_clipboard", { text: item.content });
      setToast({ message: "클립보드에 복사됨!", type: "success" });
      // In stealth mode, close after copying
      if (isStealthMode && onClose) {
        scheduleTimeout(() => onClose(), 300);
      }
    } catch (err) {
      console.error("Failed to copy:", err);
      setToast({ message: "복사 실패", type: "error" });
    }
  }, [isStealthMode, onClose, scheduleTimeout]);

  const handlePaste = useCallback(async (item: ClipboardItem) => {
    try {
      const response = await invoke<{ success: boolean; error?: { message?: string } }>("paste_text", {
        text: item.content,
      });
      if (response.success) {
        setToast({ message: "붙여넣기 완료!", type: "success" });
        // In stealth mode, close after pasting
        if (isStealthMode && onClose) {
          scheduleTimeout(() => onClose(), 100);
        }
      } else {
        setToast({
          message: response.error?.message ?? "붙여넣기 실패",
          type: "error",
        });
      }
    } catch (err) {
      console.error("Failed to paste:", err);
      setToast({ message: "붙여넣기 실패", type: "error" });
    }
  }, [isStealthMode, onClose, scheduleTimeout]);

  const handleDelete = useCallback(
    async (id: string) => {
      await deleteItem(id);
    },
    [deleteItem]
  );

  const handleTogglePin = useCallback(
    async (id: string) => {
      await togglePin(id);
    },
    [togglePin]
  );

  const handleTranslateItem = useCallback((item: ClipboardItem) => {
    if (onTranslate) {
      onTranslate(item.content);
    }
  }, [onTranslate]);

  const handlePolishItem = useCallback((item: ClipboardItem) => {
    if (onPolish) {
      onPolish(item.content);
    }
  }, [onPolish]);

  const handleEditItem = useCallback(async (item: ClipboardItem) => {
    try {
      await invoke("open_postit_editor", {
        mode: "edit",
        itemId: item.id,
      });
    } catch (err) {
      console.error("Failed to open editor:", err);
      setToast({ message: "편집기를 열 수 없습니다", type: "error" });
    }
  }, []);

  const handleMoveToMonitor = async (index: number) => {
    try {
      await invoke("move_to_monitor", { monitorIndex: index, anchor: "bottom" });
      setCurrentMonitor(index);

      // Update lastSavedWidthRef with the new window width after monitor change
      scheduleTimeout(async () => {
        try {
          const window = getCurrentWindow();
          const size = await window.outerSize();
          const scaleFactor = await window.scaleFactor();
          lastSavedWidthRef.current = Math.round(size.width / scaleFactor);
        } catch (err) {
          console.error("Failed to update lastSavedWidth:", err);
        }
      }, 100);
    } catch (err) {
      console.error("Failed to move window:", err);
    }
  };

  const handleToggleCollapse = async () => {
    if (drawerMode === "collapsed") {
      await updateDrawerMode("expanded");
    } else {
      await updateDrawerMode("collapsed");
    }
  };

  const handleOpenSettings = useCallback(async () => {
    setCurrentView("settings");
    await updateDrawerMode("full");
  }, [updateDrawerMode]);

  const handleOpenGlossary = async () => {
    setCurrentView("glossary");
    await updateDrawerMode("full");
  };

  const handleBackToHistory = async () => {
    setCurrentView("history");
    await updateDrawerMode("expanded");
  };

  // Open settings when requested by App-level menu events.
  useEffect(() => {
    if (!openSettingsSignal) {
      return;
    }

    void handleOpenSettings();
  }, [openSettingsSignal, handleOpenSettings]);


  // Horizontal scroll with mouse wheel
  const handleWheel = (e: React.WheelEvent) => {
    if (scrollRef.current) {
      scrollRef.current.scrollLeft += e.deltaY;
    }
  };

  const pinnedItems = items.filter((item) => item.isPinned);
  const unpinnedItems = items.filter((item) => !item.isPinned);
  const sortedItems = [...pinnedItems, ...unpinnedItems];

  const isCollapsed = drawerMode === "collapsed";
  const showBackButton = currentView !== "history";

  return (
    <div className="panel">
      {/* Toast */}
      {toast && (
        <Toast message={toast.message} type={toast.type} onClose={() => setToast(null)} />
      )}

      <div className="grabber" aria-hidden="true" />

      {/* Toolbar - draggable area */}
      <header className="toolbar" onMouseDown={handleDragStart}>
        {showBackButton && (
          <div className="flex items-center gap-2.5">
            <IconButton
              icon="chevron-left"
              label="뒤로"
              variant="glass"
              iconSize={18}
              onClick={handleBackToHistory}
            />
            <h1 className="text-title text-label">
              {currentView === "settings" ? "설정" : "용어집"}
            </h1>
          </div>
        )}

        {/* Search (history) */}
        {currentView === "history" && (
          <div className="relative w-[260px] shrink">
            <label htmlFor="drawer-history-search" className="sr-only">
              클립보드 히스토리 검색
            </label>
            <Icon
              name="search"
              size={14}
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-label-3"
            />
            <input
              id="drawer-history-search"
              type="text"
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              aria-label="클립보드 히스토리 검색"
              placeholder="검색"
              className="field field-search"
            />
          </div>
        )}

        {/* Glossary controls are portaled into this slot by GlossaryList */}
        {currentView === "glossary" && (
          <div ref={setToolbarSlot} className="flex min-w-0 flex-1 items-center gap-2.5" />
        )}

        {/* Monitor selector - show only when multiple monitors are connected */}
        {monitors.length > 1 && (
          <div className="flex items-center gap-2">
            <Icon name="display" size={15} className="text-label-3" />
            <Segmented
              ariaLabel="모니터 선택"
              value={currentMonitor}
              options={monitors.map((_, index) => ({
                value: index,
                label: String(index + 1),
                title: `모니터 ${index + 1}로 이동 (⌥${index + 1})`,
              }))}
              onChange={handleMoveToMonitor}
            />
          </div>
        )}

        {/* Accessibility warning */}
        {hasAccessibility === false && (
          <Button
            size="sm"
            variant="tinted"
            tone="orange"
            icon="warning"
            title="단축키(⌘CC, ⌘EE)를 쓰려면 접근성 권한이 필요합니다"
            onClick={() => {
              void invoke("open_accessibility_settings").catch((err) => {
                console.error("Failed to open accessibility settings:", err);
                setToast({ message: "접근성 설정을 열 수 없습니다", type: "error" });
              });
            }}
          >
            접근성 권한 필요
          </Button>
        )}

        {/* Hotkey hints - only in history view */}
        {currentView === "history" && (
          <div className="hidden shrink-0 items-center gap-3 text-caption text-label-3 md:flex">
            <ShortcutHint keys="⌘CC" label="번역" title="선택한 텍스트 번역 (⌘C 두 번)" />
            <ShortcutHint keys="⌘EE" label="다듬기" title="선택한 텍스트 다듬기 (⌘E 두 번)" />
            <ShortcutHint keys="⌘⌥V" label="히스토리" title="클립보드 히스토리 열기" />
          </div>
        )}

        <div className="ml-auto flex shrink-0 items-center gap-2.5">
          {currentView === "history" && (
            <>
              <ShortcutsPopover />

              {/* Item count */}
              <span className="min-w-[2.5em] text-right text-caption tabular-nums text-label-3">
                {items.length}개
              </span>

              <div className="cluster">
                <IconButton icon="book" label="용어집" onClick={handleOpenGlossary} />
                <IconButton icon="gear" label="설정" onClick={handleOpenSettings} />
              </div>
            </>
          )}

          {/* Collapse button - only show when not in stealth mode */}
          {!isStealthMode && (
            <button
              type="button"
              onClick={handleToggleCollapse}
              className="icon-btn icon-btn-glass"
              aria-label={isCollapsed ? "펼치기" : "접기"}
              title={isCollapsed ? "펼치기" : "접기"}
            >
              <Icon
                name="chevron-down"
                size={16}
                className={`transition-transform duration-200 ${isCollapsed ? "rotate-180" : ""}`}
              />
            </button>
          )}

          {/* Close button - only show in stealth mode */}
          {isStealthMode && onClose && (
            <IconButton
              icon="xmark"
              label="닫기 (ESC)"
              variant="glass"
              onClick={onClose}
            />
          )}
        </div>
      </header>

      {/* Content */}
      {!isCollapsed && (
        <div key={currentView} className="animate-fade-in min-h-0 flex-1 overflow-hidden">
          {currentView === "history" && (
            <div className="relative h-full">
              <p id="history-scroll-hint" className="sr-only">
                좌우로 스크롤
              </p>
              <div
                ref={scrollRef}
                onWheel={handleWheel}
                aria-describedby="history-scroll-hint"
                className="scroll-fade-x flex h-full scroll-smooth overflow-x-auto overflow-y-hidden px-4 py-4"
                style={{ gap: CARD_GAP }}
              >
                {/* Create new post-it card - always shown first */}
                <CreatePostItCard onClick={handleCreateNewItem} />

                {isLoading && items.length === 0 ? (
                  <div
                    className="flex flex-1 items-center justify-center gap-2 text-body text-label-3"
                    role="status"
                    aria-live="polite"
                  >
                    <Spinner size={16} />
                    <span>히스토리를 불러오는 중…</span>
                  </div>
                ) : items.length === 0 ? (
                  <div className="flex min-w-0 flex-1 flex-col items-center justify-center text-center">
                    <Icon name="clipboard" size={30} strokeWidth={1.3} className="text-label-3" />
                    <p className="mt-2.5 text-body font-medium text-label-2">
                      {searchQuery ? "검색 결과가 없습니다" : "새 메모를 만들어보세요"}
                    </p>
                    <p className="mt-0.5 text-sub text-label-3">
                      {searchQuery
                        ? "다른 검색어를 입력해 보세요."
                        : "복사한 텍스트는 자동으로 이곳에 쌓입니다."}
                    </p>
                  </div>
                ) : (
                  sortedItems.map((item, index) => (
                    <PostItCard
                      key={item.id}
                      item={item}
                      index={index}
                      onCopy={handleCopy}
                      onPaste={handlePaste}
                      onDelete={handleDelete}
                      onTogglePin={handleTogglePin}
                      onTranslate={onTranslate ? handleTranslateItem : undefined}
                      onPolish={onPolish ? handlePolishItem : undefined}
                      onEdit={handleEditItem}
                      showPasteButton={isStealthMode}
                    />
                  ))
                )}
              </div>
            </div>
          )}

          {currentView === "settings" && (
            <div className="h-full">
              <SettingsPanel />
            </div>
          )}

          {currentView === "glossary" && (
            <div className="h-full">
              <GlossaryList toolbarSlot={toolbarSlot} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
