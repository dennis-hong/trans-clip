/// Build the system prompt for polishing text
pub fn build_system_prompt() -> String {
    r#"글을 다듬어주는 도우미입니다. 글쓴이의 문체와 의도를 살리면서 표현만 자연스럽게 다듬어주세요.

- 원문의 내용, 구조, 분량을 유지하세요. 새로운 내용을 추가하거나 흐름을 재구성하지 마세요.
- 원문에 없는 형식(불릿, 헤딩 등)을 추가하지 마세요.
- 다듬어진 결과만 출력하세요. 설명이나 주석을 붙이지 마세요."#.to_string()
}

/// Build the English system prompt for polishing text
pub fn build_system_prompt_english() -> String {
    r#"You are a writing assistant that polishes text. Preserve the author's voice and intent while refining the expression.

- Keep the original content, structure, and length. Do not add new content or reorganize the flow.
- Do not add formatting (bullets, headings, etc.) that wasn't in the original.
- Output only the polished result. No explanations or comments."#.to_string()
}

/// Get a brief hint for the context type
pub fn get_context_description(context: &str, lang: &str) -> String {
    if lang == "ko" {
        match context {
            "report-to-superior" => "상사/임원에게 보고".to_string(),
            "team-announcement" => "팀 공지".to_string(),
            "peer-discussion" => "동료와 논의".to_string(),
            "external-formal" => "외부 공식 소통".to_string(),
            "documentation" => "기술 문서 작성".to_string(),
            _ => String::new(),
        }
    } else {
        match context {
            "report-to-superior" => "reporting to a manager/executive".to_string(),
            "team-announcement" => "team announcement".to_string(),
            "peer-discussion" => "discussion with colleagues".to_string(),
            "external-formal" => "formal external communication".to_string(),
            "documentation" => "technical documentation".to_string(),
            _ => String::new(),
        }
    }
}

/// Get a brief hint for the channel type
pub fn get_channel_description(channel: &str, lang: &str) -> String {
    if lang == "ko" {
        match channel {
            "slack-message" => "슬랙 메시지".to_string(),
            "slack-thread" => "슬랙 스레드 답글".to_string(),
            "confluence-wiki" => "컨플루언스 위키".to_string(),
            "jira-comment" => "Jira 코멘트".to_string(),
            "jira-description" => "Jira 이슈 설명".to_string(),
            "email" => "업무 이메일".to_string(),
            "pr-description" => "PR 설명".to_string(),
            "code-review" => "코드 리뷰 코멘트".to_string(),
            _ => String::new(),
        }
    } else {
        match channel {
            "slack-message" => "Slack message".to_string(),
            "slack-thread" => "Slack thread reply".to_string(),
            "confluence-wiki" => "Confluence wiki".to_string(),
            "jira-comment" => "Jira comment".to_string(),
            "jira-description" => "Jira issue description".to_string(),
            "email" => "business email".to_string(),
            "pr-description" => "PR description".to_string(),
            "code-review" => "code review comment".to_string(),
            _ => String::new(),
        }
    }
}

/// Build a compact options hint from selected options
pub fn build_options_section(options: &[String], lang: &str) -> String {
    if options.is_empty() {
        return String::new();
    }

    let labels: Vec<&str> = options
        .iter()
        .filter_map(|opt| {
            if lang == "ko" {
                match opt.as_str() {
                    "shorter" => Some("더 짧게"),
                    "longer" => Some("더 자세하게"),
                    "bullet" => Some("불릿으로 정리"),
                    "formal" => Some("더 격식있게"),
                    "casual" => Some("더 캐주얼하게"),
                    "action-clear" => Some("액션 명확히"),
                    _ => None,
                }
            } else {
                match opt.as_str() {
                    "shorter" => Some("make it shorter"),
                    "longer" => Some("more detail"),
                    "bullet" => Some("use bullet points"),
                    "formal" => Some("more formal"),
                    "casual" => Some("more casual"),
                    "action-clear" => Some("clarify actions"),
                    _ => None,
                }
            }
        })
        .collect();

    if labels.is_empty() {
        return String::new();
    }

    format!(" [{}]", labels.join(", "))
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

    let mut hints = Vec::new();
    if !context_desc.is_empty() {
        hints.push(context_desc);
    }
    if !channel_desc.is_empty() {
        hints.push(channel_desc);
    }

    let hint_line = if hints.is_empty() {
        String::new()
    } else if detected_lang == "ko" {
        format!("({}) ", hints.join(", "))
    } else {
        format!("({}) ", hints.join(", "))
    };

    if detected_lang == "ko" {
        format!(
            "{hint_line}아래 글을 다듬어주세요.{options_section}\n\n{text}",
            hint_line = hint_line,
            options_section = options_section,
            text = text,
        )
    } else {
        format!(
            "{hint_line}Please polish the following text.{options_section}\n\n{text}",
            hint_line = hint_line,
            options_section = options_section,
            text = text,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{build_options_section, build_user_prompt};

    #[test]
    fn options_section_is_empty_when_no_options() {
        let section = build_options_section(&[], "ko");
        assert_eq!(section, "");
    }

    #[test]
    fn options_section_contains_selected_option_text() {
        let options = vec!["shorter".to_string(), "formal".to_string()];
        let section = build_options_section(&options, "en");

        assert!(section.contains("make it shorter"));
        assert!(section.contains("more formal"));
    }

    #[test]
    fn build_user_prompt_contains_context_channel_and_text() {
        let options = vec!["action-clear".to_string()];
        let prompt = build_user_prompt(
            "Please review this update.",
            "peer-discussion",
            "slack-message",
            &options,
            "en",
        );

        assert!(prompt.contains("discussion with colleagues"));
        assert!(prompt.contains("Slack message"));
        assert!(prompt.contains("Please review this update."));
        assert!(prompt.contains("clarify actions"));
    }
}
