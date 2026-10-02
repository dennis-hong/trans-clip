import { Icon, Kbd } from "@/components/common";

interface CreatePostItCardProps {
  onClick: () => void;
}

export function CreatePostItCard({ onClick }: CreatePostItCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label="새 메모 만들기"
      className="note-create group flex h-full w-[148px] shrink-0 flex-col items-center justify-center gap-2.5"
    >
      <span className="flex h-10 w-10 items-center justify-center rounded-full bg-fill text-label-2 transition-all duration-200 ease-out group-hover:scale-105 group-hover:bg-accent group-hover:text-white">
        <Icon name="plus" size={20} strokeWidth={2} />
      </span>
      <span className="text-body font-medium text-label-2 transition-colors group-hover:text-label">
        새 메모
      </span>
      <Kbd aria-hidden="true">⌘N</Kbd>
    </button>
  );
}
