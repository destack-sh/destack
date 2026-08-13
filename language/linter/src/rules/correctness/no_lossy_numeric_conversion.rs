use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow narrowing conversions whose input range is not proven to fit.
    pub NO_LOSSY_NUMERIC_CONVERSION {
        id: "no-lossy-numeric-conversion",
        summary: "Disallow narrowing conversions whose input range is not proven to fit",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
