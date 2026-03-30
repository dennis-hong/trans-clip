import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Event as TauriEvent } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/store";
import { useWindowDrag } from "@/hooks/useWindowDrag";
import { PostItCard } from "./PostItCard";
import { CreatePostItCard } from "./CreatePostItCard";
import { Toast } from "@/components/common";
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

// Hotkey hint component with custom tooltip
function HotkeyHint({ 
  keys, 
  description, 
  variant = "default" 
}: { 
  keys: string; 
  description: string; 
  variant?: "default" | "blue" | "purple";
}) {
  const baseClass = "font-mono px-1 rounded cursor-help relative group";
  const variantClass = {
    default: "bg-gray-100",
    blue: "bg-blue-100 text-blue-600",
    purple: "bg-purple-100 text-purple-600",
  }[variant];

  return (
    <span className={`${baseClass} ${variantClass}`}>
      {keys}
      <span className="absolute top-full left-1/2 -translate-x-1/2 mt-2 px-2 py-1 text-[10px] text-white bg-gray-800 rounded shadow-lg whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none z-50">
        {description}
        <span className="absolute bottom-full left-1/2 -translate-x-1/2 border-4 border-transparent border-b-gray-800" />
      </span>
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
  }, [fetchHistory]);

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
              // Card width (w-48 = 192px) + gap (gap-4 = 16px) = 208px
              const scrollAmount = 208;
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
  }, [monitors.length, currentView, isStealthMode, onClose, onTranslate, onPolish, handleCreateNewItem, scheduleTimeout]);

  const loadMonitors = async () => {
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
  };

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
    <div className="flex flex-col w-full h-full bg-gradient-to-b from-gray-50/95 to-white/95 backdrop-blur-md rounded-t-2xl border border-gray-200/50 border-b-0 shadow-2xl">
      {/* Toast */}
      {toast && (
        <Toast message={toast.message} type={toast.type} onClose={() => setToast(null)} />
      )}

      {/* Header - Draggable area */}
      <div
        className="flex items-center gap-3 px-4 py-2 cursor-move select-none border-b border-gray-200/50"
        onMouseDown={handleDragStart}
      >
        {/* Back button or Drag handle */}
        {showBackButton ? (
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleBackToHistory();
            }}
            className="p-1 rounded-lg hover:bg-gray-200/80 transition-colors"
            title="뒤로"
          >
            <svg className="w-5 h-5 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
        ) : (
          <div className="flex gap-0.5">
            <div className="w-1 h-4 bg-gray-300 rounded-full" />
            <div className="w-1 h-4 bg-gray-300 rounded-full" />
            <div className="w-1 h-4 bg-gray-300 rounded-full" />
          </div>
        )}

        {/* Title or Search */}
        {currentView === "history" ? (
          <div className="relative flex-1 max-w-xs">
            <label htmlFor="drawer-history-search" className="sr-only">
              클립보드 히스토리 검색
            </label>
            <svg
              className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              id="drawer-history-search"
              type="text"
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              aria-label="클립보드 히스토리 검색"
              placeholder="검색..."
              className="w-full pl-8 pr-3 py-1.5 bg-white/80 rounded-lg border border-gray-200 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
              onClick={(e) => e.stopPropagation()}
            />
          </div>
        ) : (
          <span className="font-medium text-gray-800">
            {currentView === "settings" ? "설정" : "용어집"}
          </span>
        )}

        {/* Monitor selector - show only when multiple monitors are connected */}
        {monitors.length > 1 && (
          <div className="flex items-center gap-1 px-2 py-1 bg-white/80 rounded-lg border border-gray-200">
            <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
              />
            </svg>
            {monitors.map((_, index) => (
              <button
                key={index}
                onClick={(e) => {
                  e.stopPropagation();
                  handleMoveToMonitor(index);
                }}
                className={`
                  w-6 h-6 text-xs font-medium rounded transition-colors
                  ${currentMonitor === index
                    ? "bg-blue-500 text-white"
                    : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                  }
                `}
                title={`모니터 ${index + 1}로 이동 (⌥${index + 1})`}
              >
                {index + 1}
              </button>
            ))}
          </div>
        )}

        {/* Spacer */}
        <div className="flex-1" />

        {/* Accessibility warning icon */}
        {hasAccessibility === false && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              void invoke("open_accessibility_settings").catch((err) => {
                console.error("Failed to open accessibility settings:", err);
                setToast({ message: "접근성 설정을 열 수 없습니다", type: "error" });
              });
            }}
            className="p-1.5 rounded-lg bg-amber-100 hover:bg-amber-200 transition-colors"
            title="접근성 권한 필요"
          >
            <svg className="w-4 h-4 text-amber-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </button>
        )}

        {/* Hotkey hints - only in history view */}
        {currentView === "history" && (
          <div className="hidden sm:flex items-center gap-1 text-[10px] text-gray-400">
            <HotkeyHint keys="⌘CC" description="선택한 텍스트 번역 (Cmd+C 두 번)" />
            <HotkeyHint keys="⌘EE" description="선택한 텍스트 다듬기 (Cmd+E 두 번)" />
            <HotkeyHint keys="⌘⌥V" description="클립보드 히스토리 열기" />
            {isStealthMode && (
              <>
                <HotkeyHint keys="1-9" description="N번째 항목 클립보드에 복사" />
                <HotkeyHint keys="⇧N" description="Shift+숫자: N번째 항목 번역" variant="blue" />
                <HotkeyHint keys="⌃N" description="Ctrl+숫자: N번째 항목 다듬기" variant="purple" />
              </>
            )}
            <HotkeyHint keys="←→" description="좌우 화살표로 스크롤" />
          </div>
        )}

        {/* Item count - only in history view */}
        {currentView === "history" && (
          <span className="text-xs text-gray-500 tabular-nums">
            {items.length}개
          </span>
        )}

        {/* Glossary button - only in history view */}
        {currentView === "history" && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleOpenGlossary();
            }}
            className="p-1.5 rounded-lg hover:bg-gray-200/80 transition-colors"
            title="용어집"
          >
            <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
            </svg>
          </button>
        )}

        {/* Settings button - only in history view */}
        {currentView === "history" && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleOpenSettings();
            }}
            className="p-1.5 rounded-lg hover:bg-gray-200/80 transition-colors"
            title="설정"
          >
            <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        )}

        {/* Collapse button - only show when not in stealth mode */}
        {!isStealthMode && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleToggleCollapse();
            }}
            className="p-1.5 rounded-lg hover:bg-gray-200/80 transition-colors"
            title={isCollapsed ? "펼치기" : "접기"}
          >
            <svg
              className={`w-4 h-4 text-gray-500 transition-transform duration-200 ${
                isCollapsed ? "rotate-180" : ""
              }`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </button>
        )}

        {/* Close button - only show in stealth mode */}
        {isStealthMode && onClose && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            className="p-1.5 rounded-lg hover:bg-gray-200/80 transition-colors"
            title="닫기 (ESC)"
          >
            <svg
              className="w-4 h-4 text-gray-500"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>

      {/* Content */}
      {!isCollapsed && (
        <div className="flex-1 overflow-hidden">
          {currentView === "history" && (
            <div className="relative h-full">
              <p
                id="history-scroll-hint"
                className="pointer-events-none absolute top-2 right-4 z-10 rounded-full border border-gray-200 bg-white/85 px-2 py-0.5 text-[10px] text-gray-500 shadow-sm"
              >
                좌우로 스크롤
              </p>
              <div
                ref={scrollRef}
                onWheel={handleWheel}
                aria-describedby="history-scroll-hint"
                className="h-full flex items-start gap-4 px-4 py-3 overflow-x-auto overflow-y-hidden scroll-smooth"
                style={{
                  scrollbarWidth: "thin",
                  scrollbarColor: "rgba(156, 163, 175, 0.5) transparent",
                }}
              >
                {/* Create new post-it card - always shown first */}
                <CreatePostItCard onClick={handleCreateNewItem} />

                {isLoading && items.length === 0 ? (
                  <div className="flex items-center justify-center w-full py-8" role="status" aria-live="polite">
                    <div className="flex items-center gap-2 text-sm text-blue-600">
                      <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
                      <span>히스토리를 불러오는 중...</span>
                    </div>
                  </div>
                ) : items.length === 0 ? (
                  <div className="flex flex-col items-center justify-center flex-1 py-8 text-center">
                    <svg
                      className="w-12 h-12 text-gray-300"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={1.5}
                        d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                      />
                    </svg>
                    <p className="mt-2 text-sm text-gray-500">
                      {searchQuery ? "검색 결과가 없습니다" : "새 메모를 만들어보세요"}
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
            <div className="h-full overflow-y-auto">
              <SettingsPanel />
            </div>
          )}

          {currentView === "glossary" && (
            <div className="h-full overflow-y-auto">
              <GlossaryList />
            </div>
          )}
        </div>
      )}

    </div>
  );
}
