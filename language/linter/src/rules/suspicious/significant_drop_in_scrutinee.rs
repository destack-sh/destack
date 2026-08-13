use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow scrutinees that unnecessarily extend significant destruction.
    pub SIGNIFICANT_DROP_IN_SCRUTINEE {
        id: "significant-drop-in-scrutinee",
        summary: "Disallow scrutinees that unnecessarily extend significant destruction",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
