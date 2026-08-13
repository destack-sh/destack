use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer conversions over field-by-field reconstruction.
    pub MANUAL_STRUCT_CONVERSION {
        id: "manual-struct-conversion",
        summary: "Prefer conversions over field-by-field reconstruction",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
