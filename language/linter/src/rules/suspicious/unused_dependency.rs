use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow declared dependencies unused by the target program.
    pub UNUSED_DEPENDENCY {
        id: "unused-dependency",
        summary: "Disallow declared dependencies unused by the target program",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram,
    }
}
