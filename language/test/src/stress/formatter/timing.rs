use std::time::Duration;

use super::document::FormatterDocumentStats;

/// Formatter stress phase timing for one format pass.
#[derive(Debug, Clone, Copy)]
pub(super) struct FormatterTiming {
    /// Parser construction and expression parsing.
    pub(super) parse: Duration,
    /// Parser diagnostic construction.
    pub(super) diagnostics: Duration,
    /// Parser comment attachment.
    pub(super) comments: Duration,
    /// Token span, side token span, and side span materialization.
    pub(super) token_spans: Duration,
    /// Parent index construction.
    pub(super) parents: Duration,
    /// Formatter option conversion.
    pub(super) options: Duration,
    /// Formatter context construction.
    pub(super) context: Duration,
    /// Formatter document construction.
    pub(super) format: Duration,
    /// Formatter document printing.
    pub(super) print: Duration,
    /// Formatter document shape after construction.
    pub(super) document: FormatterDocumentStats,
}

impl FormatterTiming {
    /// Format these phase timings for terminal output.
    pub(super) fn format(self) -> String {
        format!(
            "parse {}, diagnostics {}, comments {}, token spans {}, parents {}, options {}, context {}, format {}, print {}, document {}",
            format_duration(self.parse),
            format_duration(self.diagnostics),
            format_duration(self.comments),
            format_duration(self.token_spans),
            format_duration(self.parents),
            format_duration(self.options),
            format_duration(self.context),
            format_duration(self.format),
            format_duration(self.print),
            self.document.format()
        )
    }
}

/// Format a short elapsed duration.
pub(super) fn format_duration(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}
