use crate::LabeledSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuggestionStyle {
    Normal,
    Short,
    Hidden,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Applicability {
    /// The suggestion is machine-applicable.
    Automatic,
    /// The suggestion is maybe incorrect.
    Dangerous,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Suggestion {
    /// The spans involved in the replacement.
    pub spans: Vec<LabeledSpan>,
    /// The replacement string.
    pub replacement: Option<String>,
    /// The message of the suggestion.
    pub message: String,
    /// The style of the suggestion.
    pub style: SuggestionStyle,
    /// The applicability of the suggestion.
    pub applicability: Applicability,
}
