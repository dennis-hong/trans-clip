import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  PolishContext,
  PolishChannel,
  PolishOption,
  PolishContextInfo,
  PolishChannelInfo,
  PolishOptionInfo,
} from "@/types";

// ============================================
// Constants - 기본 제공 상황/채널/옵션 정보
// ============================================

export const POLISH_CONTEXTS: PolishContextInfo[] = [
  {
    id: "report-to-superior",
    name: "상사에게 보고",
    description: "존댓말, 핵심 먼저, 결론-근거 순서",
  },
  {
    id: "team-announcement",
    name: "팀 공지",
    description: "친근하면서 명확, 불릿포인트로 정리",
  },
  {
    id: "peer-discussion",
    name: "동료와 논의",
    description: "편하게, 논의 포인트 정리",
  },
  {
    id: "external-formal",
    name: "외부 커뮤니케이션",
    description: "격식체, 정중함, 배경-목적-요청 구조",
  },
  {
    id: "documentation",
    name: "문서 작성",
    description: "객관적, 3인칭, 단계별 설명",
  },
];

export const POLISH_CHANNELS: PolishChannelInfo[] = [
  {
    id: "slack-message",
    name: "슬랙 메시지",
    description: "짧고 간결, 이모지 OK",
  },
  {
    id: "slack-thread",
    name: "슬랙 스레드",
    description: "컨텍스트 유지, 약간 더 상세",
  },
  {
    id: "confluence-wiki",
    name: "컨플루언스 위키",
    description: "헤딩/불릿 구조화, 완전한 문장",
  },
  {
    id: "jira-comment",
    name: "Jira 코멘트",
    description: "간결, 결론과 액션 중심",
  },
  {
    id: "jira-description",
    name: "Jira 설명",
    description: "배경-목표-상세-AC 구조",
  },
  {
    id: "email",
    name: "이메일",
    description: "인사-본문-마무리 구조",
  },
  {
    id: "pr-description",
    name: "PR 설명",
    description: "What-Why-How 구조",
  },
  {
    id: "code-review",
    name: "코드 리뷰",
    description: "건설적, 구체적 제안 포함",
  },
];

export const POLISH_OPTIONS: PolishOptionInfo[] = [
  {
    id: "shorter",
    name: "더 짧게",
    description: "핵심만 남기고 불필요한 부분 제거",
  },
  {
    id: "longer",
    name: "더 자세하게",
    description: "부연 설명과 맥락 추가",
  },
  {
    id: "bullet",
    name: "불릿으로 정리",
    description: "나열된 내용을 불릿포인트로 구조화",
  },
  {
    id: "formal",
    name: "더 격식있게",
    description: "톤을 높여 공식적으로",
  },
  {
    id: "casual",
    name: "더 캐주얼하게",
    description: "톤을 낮춰 편하게",
  },
  {
    id: "action-clear",
    name: "액션 명확히",
    description: "요청사항/다음 단계를 명확하게",
  },
];

// ============================================
// Store
// ============================================

interface PolishStore {
  // 마지막으로 사용한 설정 (persist)
  lastContext: PolishContext;
  lastChannel: PolishChannel;
  lastOptions: PolishOption[];

  // Actions
  setLastContext: (context: PolishContext) => void;
  setLastChannel: (channel: PolishChannel) => void;
  setLastOptions: (options: PolishOption[]) => void;
  toggleOption: (option: PolishOption) => void;
  resetToDefaults: () => void;
}

const DEFAULT_CONTEXT: PolishContext = "peer-discussion";
const DEFAULT_CHANNEL: PolishChannel = "slack-message";
const DEFAULT_OPTIONS: PolishOption[] = [];

export const usePolishStore = create<PolishStore>()(
  persist(
    (set, get) => ({
      lastContext: DEFAULT_CONTEXT,
      lastChannel: DEFAULT_CHANNEL,
      lastOptions: DEFAULT_OPTIONS,

      setLastContext: (context: PolishContext) => {
        set({ lastContext: context });
      },

      setLastChannel: (channel: PolishChannel) => {
        set({ lastChannel: channel });
      },

      setLastOptions: (options: PolishOption[]) => {
        set({ lastOptions: options });
      },

      toggleOption: (option: PolishOption) => {
        const current = get().lastOptions;
        if (current.includes(option)) {
          set({ lastOptions: current.filter((o) => o !== option) });
        } else {
          set({ lastOptions: [...current, option] });
        }
      },

      resetToDefaults: () => {
        set({
          lastContext: DEFAULT_CONTEXT,
          lastChannel: DEFAULT_CHANNEL,
          lastOptions: DEFAULT_OPTIONS,
        });
      },
    }),
    {
      name: "polish-settings",
    }
  )
);
