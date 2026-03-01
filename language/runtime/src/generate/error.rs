/// Validation issue emitted by generator model checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratorValidationIssue {
    /// Stable issue code.
    pub code: &'static str,
    /// Domain associated with the issue.
    pub domain: String,
    /// Optional binding key for the issue.
    pub binding: Option<String>,
    /// Human-readable issue message.
    pub message: String,
}

impl GeneratorValidationIssue {
    /// Build one issue for one domain-level invariant.
    pub(crate) fn domain(code: &'static str, domain: &str, message: impl Into<String>) -> Self {
        Self {
            code,
            domain: domain.to_string(),
            binding: None,
            message: message.into(),
        }
    }

    /// Build one issue for one binding-level invariant.
    pub(crate) fn binding(
        code: &'static str,
        domain: &str,
        binding: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            domain: domain.to_string(),
            binding: Some(binding.to_string()),
            message: message.into(),
        }
    }
}
