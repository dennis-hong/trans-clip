import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/store";
import { PostItCard } from "./PostItCard";
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

interface DrawerPanelProps {
  hasAccessibility?: boolean | null;
}

export function DrawerPanel({ hasAccessibility }: DrawerPanelProps) {
  const [currentView, setCurrentView] = useState<DrawerView>("history");
  const [drawerMode, setDrawerMode] = useState<DrawerMode>("expanded");
  const [searchQuery, setSearchQuery] = useState("");
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [currentMonitor, setCurrentMonitor] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const scrollRef = useRef<HTMLDivElement>(null);

  const { items, isLoading, fetchHistory, deleteItem, togglePin } = useClipboardStore();

  // Fetch history on mount
  useEffect(() => {
    fetchHistory();
    loadMonitors();
  }, [fetchHistory]);

  // Listen for clipboard changes
  useEffect(() => {
    const unlisten = listen<ClipboardChangedPayload>("clipboard_changed", () => {
      fetchHistory();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchHistory]);

  // Listen for open_settings event from tray menu
  useEffect(() => {
    const unlisten = listen("open_settings", () => {
      handleOpenSettings();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Keyboard shortcuts for monitor switching (Alt+1, Alt+2, Alt+3)
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      // Alt/Option + number keys (1, 2, 3)
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
          } catch (err) {
            console.error(`Failed to move to monitor ${monitorIndex + 1}:`, err);
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [monitors.length]);

  const loadMonitors = async () => {
    try {
      // Get monitors list (already sorted by position - left to right)
      const result = await invoke<MonitorInfo[]>("get_monitors");
      setMonitors(result);
      
      // Get current monitor index based on window position
      const currentIdx = await invoke<number>("get_current_monitor_index");
      setCurrentMonitor(currentIdx);
    } catch (err) {
      console.error("Failed to get monitors:", err);
    }
  };

  const updateDrawerMode = async (mode: DrawerMode) => {
    setDrawerMode(mode);
    try {
      await invoke("set_drawer_mode", { mode });
    } catch (err) {
      console.error("Failed to set drawer mode:", err);
    }
  };

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
    } catch (err) {
      console.error("Failed to copy:", err);
      setToast({ message: "복사 실패", type: "error" });
    }
  }, []);

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

  const handleMoveToMonitor = async (index: number) => {
    try {
      await invoke("move_to_monitor", { monitorIndex: index, anchor: "bottom" });
      setCurrentMonitor(index);
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

  const handleOpenSettings = async () => {
    setCurrentView("settings");
    await updateDrawerMode("full");
  };

  const handleOpenGlossary = async () => {
    setCurrentView("glossary");
    await updateDrawerMode("full");
  };

  const handleBackToHistory = async () => {
    setCurrentView("history");
    await updateDrawerMode("expanded");
  };

  // Drag handling for window movement (only horizontal)
  const handleDragStart = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, input")) return;
    setIsDragging(true);
    setDragStart({ x: e.screenX, y: e.screenY });
  };

  const handleDragMove = useCallback(
    async (e: MouseEvent) => {
      if (!isDragging) return;

      const deltaX = e.screenX - dragStart.x;

      try {
        const pos = await invoke<{ x: number; y: number }>("get_window_position");
        await invoke("set_window_position", {
          x: pos.x + deltaX,
          y: pos.y, // Keep y position fixed
        });
        setDragStart({ x: e.screenX, y: e.screenY });
      } catch (err) {
        console.error("Failed to move window:", err);
      }
    },
    [isDragging, dragStart]
  );

  const handleDragEnd = useCallback(async () => {
    setIsDragging(false);
    try {
      await invoke("snap_to_bottom");
      // Update current monitor index after snapping
      const currentIdx = await invoke<number>("get_current_monitor_index");
      setCurrentMonitor(currentIdx);
    } catch (err) {
      console.error("Failed to snap to bottom:", err);
    }
  }, []);

  useEffect(() => {
    if (isDragging) {
      window.addEventListener("mousemove", handleDragMove);
      window.addEventListener("mouseup", handleDragEnd);
    }
    return () => {
      window.removeEventListener("mousemove", handleDragMove);
      window.removeEventListener("mouseup", handleDragEnd);
    };
  }, [isDragging, handleDragMove, handleDragEnd]);

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
              type="text"
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
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
              invoke("open_accessibility_settings");
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
            <span className="font-mono bg-gray-100 px-1 rounded">⌘CC</span>
            <span className="font-mono bg-gray-100 px-1 rounded">⌘DD</span>
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

        {/* Collapse button */}
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
      </div>

      {/* Content */}
      {!isCollapsed && (
        <div className="flex-1 overflow-hidden">
          {currentView === "history" && (
            <div
              ref={scrollRef}
              onWheel={handleWheel}
              className="h-full flex items-start gap-4 px-4 py-3 overflow-x-auto overflow-y-hidden scroll-smooth"
              style={{
                scrollbarWidth: "thin",
                scrollbarColor: "rgba(156, 163, 175, 0.5) transparent",
              }}
            >
              {isLoading && items.length === 0 ? (
                <div className="flex items-center justify-center w-full py-8">
                  <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
                </div>
              ) : items.length === 0 ? (
                <div className="flex flex-col items-center justify-center w-full py-8 text-center">
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
                    {searchQuery ? "검색 결과가 없습니다" : "클립보드 히스토리가 비어있습니다"}
                  </p>
                </div>
              ) : (
                sortedItems.map((item) => (
                  <PostItCard
                    key={item.id}
                    item={item}
                    onCopy={handleCopy}
                    onDelete={handleDelete}
                    onTogglePin={handleTogglePin}
                  />
                ))
              )}
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
