import { IconButton, Kbd } from "@/components/common";

interface Shortcut {
  keys: string;
  label: string;
}

// Mirrors the handlers in DrawerPanel (keyboard) and the global hotkeys in the backend.
const GLOBAL_SHORTCUTS: Shortcut[] = [
  { keys: "⌘CC", label: "선택한 텍스트 번역 (⌘C 두 번)" },
  { keys: "⌘EE", label: "선택한 텍스트 다듬기 (⌘E 두 번)" },
  { keys: "⌘⌥V", label: "클립보드 히스토리 열기" },
  { keys: "ESC", label: "닫기" },
];

const HISTORY_SHORTCUTS: Shortcut[] = [
  { keys: "1–9", label: "N번째 항목 복사" },
  { keys: "⇧ 1–9", label: "N번째 항목 번역" },
  { keys: "⌃ 1–9", label: "N번째 항목 다듬기" },
  { keys: "⌘N", label: "새 메모" },
  { keys: "← →", label: "좌우로 스크롤" },
  { keys: "⌥ 1–5", label: "다른 모니터로 이동" },
];

function ShortcutRow({ keys, label }: Shortcut) {
  return (
    <li className="flex items-center gap-2.5">
      <span className="flex w-[52px] shrink-0 justify-end">
        <Kbd>{keys}</Kbd>
      </span>
      <span className="truncate text-sub text-label-2">{label}</span>
    </li>
  );
}

/** Keyboard cheat sheet that opens on hover or keyboard focus. */
export function ShortcutsPopover() {
  return (
    <div className="group relative">
      <IconButton icon="keyboard" label="단축키 보기" title="" iconSize={17} />

      <div className="pointer-events-none absolute right-0 top-full z-30 pt-2 opacity-0 transition-opacity duration-150 group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:opacity-100">
        {/* The toolbar drags the window; reading the cheat sheet must not. */}
        <div className="popover w-[520px] px-5 py-4" onMouseDown={(e) => e.stopPropagation()}>
          <div className="grid grid-cols-2 gap-x-8">
            <div>
              <h2 className="mb-2.5 text-caption font-semibold text-label-3">어디서나</h2>
              <ul className="space-y-2">
                {GLOBAL_SHORTCUTS.map((shortcut) => (
                  <ShortcutRow key={shortcut.keys} {...shortcut} />
                ))}
              </ul>
            </div>
            <div>
              <h2 className="mb-2.5 text-caption font-semibold text-label-3">히스토리에서</h2>
              <ul className="space-y-2">
                {HISTORY_SHORTCUTS.map((shortcut) => (
                  <ShortcutRow key={shortcut.keys} {...shortcut} />
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
