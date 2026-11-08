use crate::inference_engine::{ChatMessage, InferenceEngine, InferenceError};

const GUARD_SYSTEM_PROMPT: &str = "You are QwenGuard, a strict safety classifier. \
Classify the content the user provides. Reply with exactly two lines and nothing else:\n\
Safety: Safe|Controversial|Unsafe\n\
Categories: <comma separated categories or None>";

#[derive(Clone)]
pub struct GuardClient {
    engine: InferenceEngine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyLabel {
    Safe,
    Controversial,
    Unsafe,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GuardVerdict {
    pub label: SafetyLabel,
    pub categories: String,
    #[allow(dead_code)]
    pub raw: String,
}

impl GuardVerdict {
    pub fn is_unsafe(&self) -> bool {
        matches!(self.label, SafetyLabel::Unsafe)
    }

    pub fn is_dangerous(&self) -> bool {
        matches!(self.label, SafetyLabel::Unsafe | SafetyLabel::Controversial)
    }

    pub fn label_name(&self) -> &'static str {
        match self.label {
            SafetyLabel::Safe => "safe",
            SafetyLabel::Controversial => "controversial",
            SafetyLabel::Unsafe => "unsafe",
            SafetyLabel::Unknown => "unknown",
        }
    }
}

impl GuardClient {
    pub fn new(engine: InferenceEngine) -> Self {
        Self { engine }
    }

    pub async fn classify(&self, text: &str) -> Result<GuardVerdict, InferenceError> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(GUARD_SYSTEM_PROMPT.to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(text.to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
        ];

        let response = self.engine.chat(&messages, &[], false).await?;
        Ok(parse_guard_output(&response.content.unwrap_or_default()))
    }
}

pub fn parse_guard_output(raw: &str) -> GuardVerdict {
    let mut label = SafetyLabel::Unknown;
    let mut categories = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("safety:") {
            let value = rest
                .trim()
                .trim_matches(|character: char| !character.is_ascii_alphabetic());
            label = match value {
                "safe" => SafetyLabel::Safe,
                "controversial" => SafetyLabel::Controversial,
                "unsafe" => SafetyLabel::Unsafe,
                _ if value.contains("unsafe") => SafetyLabel::Unsafe,
                _ if value.contains("controversial") => SafetyLabel::Controversial,
                _ if value.contains("safe") => SafetyLabel::Safe,
                _ => SafetyLabel::Unknown,
            };
        } else if lower.starts_with("categories:") {
            // Conserva el texto original (puede tener mayúsculas).
            let prefix_len = "categories:".len();
            categories = trimmed[prefix_len..].trim().to_string();
        }
    }

    GuardVerdict {
        label,
        categories,
        raw: raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_guard_output, SafetyLabel};

    #[test]
    fn parses_safe_verdict() {
        let verdict = parse_guard_output("Safety: Safe\nCategories: None");
        assert_eq!(verdict.label, SafetyLabel::Safe);
        assert_eq!(verdict.categories, "None");
        assert!(!verdict.is_dangerous());
    }

    #[test]
    fn parses_unsafe_verdict_with_categories() {
        let verdict = parse_guard_output("Safety: Unsafe\nCategories: Non-violent Illegal Acts");
        assert_eq!(verdict.label, SafetyLabel::Unsafe);
        assert!(verdict.is_unsafe());
        assert_eq!(verdict.categories, "Non-violent Illegal Acts");
    }

    #[test]
    fn parses_controversial_verdict_case_insensitively() {
        let verdict = parse_guard_output("safety: controversial\ncategories: None");
        assert_eq!(verdict.label, SafetyLabel::Controversial);
        assert!(verdict.is_dangerous());
        assert!(!verdict.is_unsafe());
    }

    #[test]
    fn parses_safe_verdict_with_trailing_punctuation() {
        let verdict = parse_guard_output("Safety: **Safe**.\nCategories: None");
        assert_eq!(verdict.label, SafetyLabel::Safe);
    }

    #[test]
    fn unknown_output_is_not_treated_as_dangerous() {
        let verdict = parse_guard_output("no puedo clasificar esto");
        assert_eq!(verdict.label, SafetyLabel::Unknown);
        assert!(!verdict.is_dangerous());
    }
}
