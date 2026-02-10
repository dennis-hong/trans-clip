import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UseWindowDragOptions {
  onDragEnd?: () => void;
}

export function useWindowDrag(options?: UseWindowDragOptions) {
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

  const handleDragStart = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, input, select, textarea")) return;
    setIsDragging(true);
    setDragStart({ x: e.screenX, y: e.screenY });
  }, []);

  const handleDragMove = useCallback(
    async (e: MouseEvent) => {
      if (!isDragging) return;

      const deltaX = e.screenX - dragStart.x;
      const deltaY = e.screenY - dragStart.y;

      try {
        const pos = await invoke<{ x: number; y: number }>("get_window_position");
        await invoke("set_window_position", {
          x: pos.x + deltaX,
          y: pos.y + deltaY,
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
      await invoke("snap_to_edge", { threshold: 50 });
    } catch (err) {
      console.error("Failed to snap to edge:", err);
    }
    options?.onDragEnd?.();
  }, [options]);

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

  return { isDragging, handleDragStart };
}
