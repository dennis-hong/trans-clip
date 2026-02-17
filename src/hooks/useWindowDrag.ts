import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UseWindowDragOptions {
  onDragEnd?: () => void;
}

interface WindowPosition {
  x: number;
  y: number;
}

export function useWindowDrag(options?: UseWindowDragOptions) {
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef<{ x: number; y: number } | null>(null);
  const windowPosRef = useRef<WindowPosition | null>(null);
  const rafIdRef = useRef<number | null>(null);

  const flushWindowPosition = useCallback(() => {
    rafIdRef.current = null;
    const pos = windowPosRef.current;
    if (!pos) return;

    void invoke("set_window_position", { x: pos.x, y: pos.y }).catch((err) => {
      console.error("Failed to move window:", err);
    });
  }, []);

  const handleDragStart = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, input, select, textarea")) return;

    setIsDragging(true);
    dragStartRef.current = { x: e.screenX, y: e.screenY };
    windowPosRef.current = null;

    void invoke<WindowPosition>("get_window_position")
      .then((pos) => {
        windowPosRef.current = { x: pos.x, y: pos.y };
      })
      .catch((err) => {
        console.error("Failed to get initial window position:", err);
      });
  }, []);

  const handleDragMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging) return;

      const dragStart = dragStartRef.current;
      if (!dragStart) return;

      const deltaX = e.screenX - dragStart.x;
      const deltaY = e.screenY - dragStart.y;
      dragStartRef.current = { x: e.screenX, y: e.screenY };

      const pos = windowPosRef.current;
      if (!pos) return;

      pos.x += deltaX;
      pos.y += deltaY;

      if (rafIdRef.current === null) {
        rafIdRef.current = window.requestAnimationFrame(flushWindowPosition);
      }
    },
    [flushWindowPosition, isDragging]
  );

  const handleDragEnd = useCallback(async () => {
    setIsDragging(false);
    dragStartRef.current = null;

    if (rafIdRef.current !== null) {
      window.cancelAnimationFrame(rafIdRef.current);
      rafIdRef.current = null;
    }

    const finalPos = windowPosRef.current;
    if (finalPos) {
      try {
        await invoke("set_window_position", { x: finalPos.x, y: finalPos.y });
      } catch (err) {
        console.error("Failed to flush final window position:", err);
      }
    }

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
      if (rafIdRef.current !== null) {
        window.cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
    };
  }, [isDragging, handleDragMove, handleDragEnd]);

  return { isDragging, handleDragStart };
}
