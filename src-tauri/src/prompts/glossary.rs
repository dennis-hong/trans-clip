/// A glossary entry as presented to the model: a term and what it means.
pub struct GlossaryTerm<'a> {
    pub keyword: &'a str,
    pub description: &'a str,
}

/// Render the glossary section shared by the translate and polish prompts.
/// Returns an empty string when there are no terms.
pub fn build_glossary_section(terms: &[GlossaryTerm<'_>], task: &str) -> String {
    if terms.is_empty() {
        return String::new();
    }

    let lines: Vec<String> = terms
        .iter()
        .map(|t| format!("- {}: {}", t.keyword, t.description))
        .collect();

    format!(
        "\nGlossary — the text uses these terms. Each description explains the term's meaning in this user's context; \
if a description names a preferred wording, use exactly that wording consistently when you {task}.\n{}\n",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::{build_glossary_section, GlossaryTerm};

    #[test]
    fn empty_glossary_renders_nothing() {
        assert_eq!(build_glossary_section(&[], "translate"), "");
    }

    #[test]
    fn glossary_lists_every_term_with_description() {
        let terms = [
            GlossaryTerm {
                keyword: "스프린트",
                description: "sprint (two-week cycle)",
            },
            GlossaryTerm {
                keyword: "LGTM",
                description: "looks good to me",
            },
        ];
        let section = build_glossary_section(&terms, "translate");

        assert!(section.contains("- 스프린트: sprint (two-week cycle)"));
        assert!(section.contains("- LGTM: looks good to me"));
        assert!(section.contains("when you translate"));
    }
}
