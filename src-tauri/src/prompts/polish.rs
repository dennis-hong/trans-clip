/// Build the system prompt for polishing text
/// This establishes the role, quality standards, and Korean writing guidelines
pub fn build_system_prompt() -> String {
    r#"당신은 한국 테크 기업에서 10년 이상 경력을 가진 커뮤니케이션 전문가입니다. 슬랙, 이메일, 기술 문서, 보고서 등 다양한 업무 글쓰기에 능숙합니다.

## 품질 기준

1. **명료성**: 한 번 읽고 바로 이해되어야 합니다
2. **간결성**: 군더더기 없이 핵심만 담아야 합니다
3. **자연스러움**: 원어민이 쓴 것처럼 자연스러워야 합니다
4. **구조화**: 논리적 흐름이 명확해야 합니다
5. **목적 적합성**: 상황과 채널에 맞는 톤과 형식이어야 합니다

## 한국어 글쓰기 교정 가이드

### 번역투 제거
- "~하는 것이 가능합니다" → "~할 수 있습니다"
- "~에 대해서" → "~에 대해" 또는 "~를"
- "~하는 것이 필요합니다" → "~해야 합니다"
- "존재합니다" → "있습니다"
- "~라는 사실" → "~는 점" 또는 생략

### 피동/사동 정리
- "진행되어집니다" → "진행됩니다"
- "검토가 되었습니다" → "검토했습니다"
- "적용이 될 예정입니다" → "적용할 예정입니다"
- "시키다" 남용 자제 → 직접 동사 사용

### 장황한 표현 간소화
- "~라고 할 수 있겠습니다" → "~입니다"
- "~라고 생각이 듭니다" → "~라고 생각합니다" 또는 "~같습니다"
- "~하는 바입니다" → "~합니다"
- "~해 주시면 감사하겠습니다" → "~해 주세요"

### 연결어 다양화
- "그리고" 반복 → "또한", "아울러", "더불어", 또는 병렬 구조 활용
- "하지만" 반복 → "다만", "그러나", "반면"
- "그래서" 반복 → "따라서", "이에", "결과적으로"

## 핵심 제약

- 원문의 핵심 의도와 정보를 반드시 보존합니다
- 원문에 없는 새로운 주장, 데이터, 의견을 추가하지 않습니다
- 상황에 맞는 인사말이나 마무리를 추가하는 것은 허용됩니다
- 결과물만 출력하고, 설명이나 주석은 포함하지 않습니다"#.to_string()
}

/// Build the English system prompt for polishing text
pub fn build_system_prompt_english() -> String {
    r#"You are a communications expert with over 10 years of experience at tech companies. You are skilled in various forms of business writing including Slack messages, emails, technical documentation, and reports.

## Quality Standards

1. **Clarity**: Should be understood on first read
2. **Conciseness**: Only essential content, no fluff
3. **Naturalness**: Should sound native
4. **Structure**: Clear logical flow
5. **Appropriateness**: Right tone and format for the context

## Core Constraints

- Preserve the original intent and information
- Do not add new claims, data, or opinions not in the original
- Adding appropriate greetings or closings is allowed
- Output only the result, no explanations or comments"#.to_string()
}

/// Get detailed description for a context type
pub fn get_context_description(context: &str, lang: &str) -> String {
    if lang == "ko" {
        match context {
            "report-to-superior" => r#"**상황**: 상사나 임원에게 업무 보고

**핵심 원칙**:
- 결론을 먼저 말하고, 근거는 그 뒤에 배치
- 존댓말을 일관되게 사용
- 숫자와 구체적 사실로 명확하게 전달

**피해야 할 것**:
- 모호한 표현 ("조금", "대략", "어느 정도")
- 과도한 겸양 ("부족하지만", "미흡하나마")
- 결론 없이 나열만 하는 것

**권장 구조**:
1. 한 줄 요약 (핵심 결론)
2. 상세 내용 (근거, 진행 상황)
3. 요청 사항 또는 다음 단계"#.to_string(),

            "team-announcement" => r#"**상황**: 팀원들에게 전달하는 공지

**핵심 원칙**:
- 친근하면서도 명확하게
- 불릿포인트로 핵심 정리
- 구체적인 행동 요청 명시

**피해야 할 것**:
- 장황한 배경 설명
- 애매한 마감일 ("가능한 빨리")
- 행동 없이 정보만 전달

**권장 구조**:
1. 핵심 공지 내용
2. 영향 범위 / 주요 변경사항
3. 필요한 액션 + 담당자/마감일"#.to_string(),

            "peer-discussion" => r#"**상황**: 동료와 아이디어를 논의하거나 피드백을 주고받는 상황

**핵심 원칙**:
- 편안한 톤 유지
- 논의 포인트를 명확히 정리
- 열린 질문으로 의견 구하기

**피해야 할 것**:
- 지나치게 격식 있는 표현
- 일방적인 주장
- 맥락 없이 갑자기 의견 제시

**권장 구조**:
1. 배경/맥락 간단히
2. 내 의견 또는 제안
3. 의견 요청 / 질문"#.to_string(),

            "external-formal" => r#"**상황**: 파트너사, 고객사 등 외부와 공식 소통

**핵심 원칙**:
- 격식체로 정중하게
- 회사를 대표한다는 인식
- 명확한 요청과 기대 사항

**피해야 할 것**:
- 내부 용어, 약어 사용
- 지나친 친근함
- 모호한 약속

**권장 구조**:
1. 인사 및 소개
2. 배경/목적
3. 구체적 요청/제안
4. 마무리 및 연락처"#.to_string(),

            "documentation" => r#"**상황**: 기술 문서, 가이드, 위키 작성

**핵심 원칙**:
- 객관적이고 3인칭으로 작성
- 단계별로 명료하게 설명
- 예시와 함께 제공

**피해야 할 것**:
- 구어체, 감정적 표현
- 암묵적 지식 가정
- 버전 정보 없는 내용

**권장 구조**:
1. 개요 (무엇을, 왜)
2. 전제조건/준비사항
3. 단계별 설명
4. 예시/참고사항"#.to_string(),

            _ => "일반적인 업무 커뮤니케이션 상황입니다. 명확하고 간결하게 작성해주세요.".to_string(),
        }
    } else {
        match context {
            "report-to-superior" => r#"**Context**: Reporting to a manager or executive

**Key Principles**:
- Lead with the conclusion, then provide supporting details
- Be direct and factual
- Use specific numbers and concrete facts

**Avoid**:
- Vague qualifiers ("somewhat", "approximately")
- Excessive hedging
- Lists without conclusions"#.to_string(),

            "team-announcement" => r#"**Context**: Team announcement

**Key Principles**:
- Be friendly yet clear
- Use bullet points for key items
- Include specific action items

**Avoid**:
- Long-winded background
- Vague deadlines ("ASAP")
- Information without action items"#.to_string(),

            "peer-discussion" => r#"**Context**: Discussion with colleagues

**Key Principles**:
- Maintain a casual tone
- Clearly state discussion points
- Ask open-ended questions

**Avoid**:
- Overly formal language
- One-sided arguments
- Opinions without context"#.to_string(),

            "external-formal" => r#"**Context**: Formal external communication

**Key Principles**:
- Use formal, polite language
- Represent your organization professionally
- Be clear about requests and expectations

**Avoid**:
- Internal jargon or abbreviations
- Excessive familiarity
- Vague commitments"#.to_string(),

            "documentation" => r#"**Context**: Technical documentation

**Key Principles**:
- Write objectively in third person
- Explain step by step clearly
- Include examples

**Avoid**:
- Colloquial or emotional language
- Assuming implicit knowledge
- Missing version information"#.to_string(),

            _ => "General business communication context. Write clearly and concisely.".to_string(),
        }
    }
}

/// Get detailed description for a channel type
pub fn get_channel_description(channel: &str, lang: &str) -> String {
    if lang == "ko" {
        match channel {
            "slack-message" => r#"**채널**: 슬랙 메시지

**형식 원칙**:
- 2-3문장 이내로 짧게
- 첫 줄에 핵심 내용
- 이모지는 의미 전달에 도움될 때만 사용

**권장 구조**:
[핵심 메시지]
[배경 - 필요시 1줄]
[구체적 요청 또는 다음 단계]"#.to_string(),

            "slack-thread" => r#"**채널**: 슬랙 스레드 답글

**형식 원칙**:
- 원글의 컨텍스트 유지
- 메시지보다 약간 더 상세하게
- 관련 링크나 참고자료 첨부 가능

**권장 구조**:
[직접적인 답변/의견]
[필요시 부연 설명]
[다음 액션 또는 질문]"#.to_string(),

            "confluence-wiki" => r#"**채널**: 컨플루언스 위키 문서

**형식 원칙**:
- 헤딩(H2, H3)으로 구조화
- 불릿/넘버링으로 정리
- 완전한 문장으로 작성

**권장 구조**:
## 개요
[목적과 범위]

## 상세 내용
[핵심 내용, 구조화하여]

## 관련 문서
[링크들]"#.to_string(),

            "jira-comment" => r#"**채널**: Jira 이슈 코멘트

**형식 원칙**:
- 간결하게, 3-5줄 이내
- 결론과 액션 중심
- 멘션으로 담당자 지정

**권장 구조**:
[결론/상태 업데이트]
[필요시 근거 1-2줄]
[다음 액션 + 담당자]"#.to_string(),

            "jira-description" => r#"**채널**: Jira 이슈 설명

**형식 원칙**:
- 배경-목표-상세-AC 구조
- 명확한 수락 기준(AC) 포함
- 링크로 관련 문서 연결

**권장 구조**:
## 배경
[왜 이 이슈가 필요한지]

## 목표
[달성하고자 하는 것]

## 상세
[구체적인 요구사항]

## 수락 기준 (AC)
- [ ] 기준 1
- [ ] 기준 2"#.to_string(),

            "email" => r#"**채널**: 업무 이메일

**형식 원칙**:
- 인사-본문-마무리 3단 구성
- 요청사항을 명확하게
- 회신 기한이 있다면 명시

**권장 구조**:
[인사]

[핵심 내용]

[요청 사항 + 기한]

[마무리 인사]"#.to_string(),

            "pr-description" => r#"**채널**: GitHub/GitLab PR 설명

**형식 원칙**:
- What-Why-How 구조
- 변경사항 요약
- 테스트 방법 포함

**권장 구조**:
## What
[무엇을 변경했는지]

## Why
[왜 변경이 필요한지]

## How
[어떻게 구현했는지]

## Test
[테스트 방법]"#.to_string(),

            "code-review" => r#"**채널**: 코드 리뷰 코멘트

**형식 원칙**:
- 건설적이고 구체적으로
- 문제점과 함께 대안 제시
- 칭찬할 점도 언급

**권장 구조**:
[구체적인 피드백]
[선택적: 대안 제시]
[선택적: 코드 예시]"#.to_string(),

            _ => "일반적인 텍스트 형식입니다. 목적에 맞게 작성해주세요.".to_string(),
        }
    } else {
        match channel {
            "slack-message" => r#"**Channel**: Slack message

**Format Principles**:
- Keep to 2-3 sentences
- Lead with the key point
- Use emoji only when it aids meaning

**Structure**:
[Key message]
[Background - if needed, 1 line]
[Specific request or next step]"#.to_string(),

            "slack-thread" => r#"**Channel**: Slack thread reply

**Format Principles**:
- Maintain context from the original post
- Slightly more detailed than a message
- Can include relevant links

**Structure**:
[Direct answer/opinion]
[Additional context if needed]
[Next action or question]"#.to_string(),

            "confluence-wiki" => r#"**Channel**: Confluence wiki document

**Format Principles**:
- Structure with headings (H2, H3)
- Use bullets/numbering
- Write in complete sentences"#.to_string(),

            "jira-comment" => r#"**Channel**: Jira issue comment

**Format Principles**:
- Keep brief, 3-5 lines
- Focus on conclusions and actions
- Use mentions to assign owners"#.to_string(),

            "jira-description" => r#"**Channel**: Jira issue description

**Format Principles**:
- Background-Goal-Details-AC structure
- Include clear acceptance criteria
- Link to related documents"#.to_string(),

            "email" => r#"**Channel**: Business email

**Format Principles**:
- Greeting-Body-Closing structure
- Be clear about requests
- Include deadlines if applicable"#.to_string(),

            "pr-description" => r#"**Channel**: GitHub/GitLab PR description

**Format Principles**:
- What-Why-How structure
- Summarize changes
- Include test instructions"#.to_string(),

            "code-review" => r#"**Channel**: Code review comment

**Format Principles**:
- Be constructive and specific
- Suggest alternatives with issues
- Acknowledge good points too"#.to_string(),

            _ => "General text format. Write appropriately for the purpose.".to_string(),
        }
    }
}

/// Build the options section based on selected options
pub fn build_options_section(options: &[String], lang: &str) -> String {
    if options.is_empty() {
        return String::new();
    }

    let mut options_desc = String::new();
    for opt in options {
        let opt_text = if lang == "ko" {
            match opt.as_str() {
                "shorter" => "**더 짧게**: 핵심만 남기고 불필요한 부분을 과감히 제거해주세요. 한 문장으로 줄일 수 있다면 그렇게 해주세요.",
                "longer" => "**더 자세하게**: 부연 설명과 맥락을 추가하여 이해를 돕되, 핵심 메시지가 묻히지 않도록 해주세요.",
                "bullet" => "**불릿으로 정리**: 나열된 내용을 불릿포인트로 구조화하고, 각 항목은 한 줄로 간결하게 작성해주세요.",
                "formal" => "**더 격식있게**: 톤을 높여 공식적으로 작성하고, 존칭과 격식체를 일관되게 사용해주세요.",
                "casual" => "**더 캐주얼하게**: 톤을 낮춰 편하게 작성하되, 핵심 내용은 명확하게 유지해주세요.",
                "action-clear" => "**액션 명확히**: 요청사항이나 다음 단계를 문서 끝에 명확하게 정리하고, 가능하면 담당자와 기한을 명시해주세요.",
                _ => "",
            }
        } else {
            match opt.as_str() {
                "shorter" => "**Make it shorter**: Remove unnecessary parts, keep only the essentials. Condense to one sentence if possible.",
                "longer" => "**Make it more detailed**: Add context and explanation to aid understanding, but don't bury the main message.",
                "bullet" => "**Use bullet points**: Structure listed content with bullets, keep each item to one concise line.",
                "formal" => "**More formal**: Elevate the tone, use formal language consistently.",
                "casual" => "**More casual**: Lower the tone for a relaxed feel, but keep the core message clear.",
                "action-clear" => "**Clarify actions**: List required actions or next steps clearly at the end, include owners and deadlines if possible.",
                _ => "",
            }
        };
        if !opt_text.is_empty() {
            options_desc.push_str("\n- ");
            options_desc.push_str(opt_text);
        }
    }

    if options_desc.is_empty() {
        return String::new();
    }

    if lang == "ko" {
        format!("\n## 추가 요청사항{}", options_desc)
    } else {
        format!("\n## Additional Requests{}", options_desc)
    }
}

/// Build the user prompt for polishing
pub fn build_user_prompt(
    text: &str,
    context: &str,
    channel: &str,
    options: &[String],
    detected_lang: &str,
) -> String {
    let context_desc = get_context_description(context, detected_lang);
    let channel_desc = get_channel_description(channel, detected_lang);
    let options_section = build_options_section(options, detected_lang);

    if detected_lang == "ko" {
        format!(
            r#"아래 텍스트를 다듬어주세요.

## 상황
{context_desc}

## 채널
{channel_desc}
{options_section}
## 원문
{text}

다듬어진 결과만 출력하세요."#,
            context_desc = context_desc,
            channel_desc = channel_desc,
            options_section = options_section,
            text = text,
        )
    } else {
        format!(
            r#"Please polish the following text.

## Context
{context_desc}

## Channel
{channel_desc}
{options_section}
## Original Text
{text}

Output only the polished result."#,
            context_desc = context_desc,
            channel_desc = channel_desc,
            options_section = options_section,
            text = text,
        )
    }
}
