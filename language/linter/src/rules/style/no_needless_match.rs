use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow matches that reconstruct their scrutinee unchanged.
    pub NO_NEEDLESS_MATCH {
        id: "no-needless-match",
        summary: "Disallow matches that reconstruct their scrutinee unchanged",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
