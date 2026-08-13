use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer inspect when mapping performs an observation and returns its input.
    pub MANUAL_INSPECT {
        id: "manual-inspect",
        summary: "Prefer inspect when mapping performs an observation and returns its input",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
