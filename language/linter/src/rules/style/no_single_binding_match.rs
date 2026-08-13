use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow matches with one irrefutable arm.
    pub NO_SINGLE_BINDING_MATCH {
        id: "no-single-binding-match",
        summary: "Disallow matches with one irrefutable arm",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
