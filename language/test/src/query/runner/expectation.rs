/// One explicit text expectation for query blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextExpectation<'a> {
    /// The actual text must match exactly after normalization.
    Exact(&'a str),
}

impl<'a> TextExpectation<'a> {
    /// Parse one explicit query block text expectation.
    pub fn parse(text: &'a str) -> Result<Self, String> {
        let text = text.trim();

        if let Some(text) = text.trim().strip_prefix("contains:") {
            return Err(format!(
                "query text expectations are exact only: replace `contains:` with an exact expectation for `{}`",
                text.trim()
            ));
        }

        Ok(Self::Exact(text))
    }

    /// Return the expected text value.
    pub fn text(&self) -> &'a str {
        match self {
            Self::Exact(text) => text,
        }
    }
}

/// Match actual text against one explicit query block expectation.
pub fn matches_text_expectation(actual: &str, expectation: TextExpectation<'_>) -> bool {
    let actual = normalize_text(actual);
    let expected = normalize_text(expectation.text());

    match expectation {
        TextExpectation::Exact(_) => actual == expected,
    }
}

/// Describe one explicit query block expectation in failure output.
pub fn describe_text_expectation(expectation: TextExpectation<'_>) -> &'static str {
    match expectation {
        TextExpectation::Exact(_) => "equal",
    }
}

/// Normalize whitespace for explicit text expectations.
fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{TextExpectation, describe_text_expectation, matches_text_expectation};

    #[test]
    fn test_matches_text_expectation_uses_exact_matching_by_default() {
        assert!(!matches_text_expectation(
            "type UserId",
            TextExpectation::parse("UserId").unwrap(),
        ));
    }

    #[test]
    fn test_parse_rejects_contains_expectations() {
        let error = TextExpectation::parse("contains: UserId").unwrap_err();
        assert!(error.contains("exact only"));
    }

    #[test]
    fn test_describe_text_expectation_reports_match_mode() {
        assert_eq!(
            describe_text_expectation(TextExpectation::parse("value").unwrap()),
            "equal"
        );
    }
}
