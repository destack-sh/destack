use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer optional chaining to equivalent guarded property access.
    pub PREFER_OPTIONAL_CHAIN {
        id: "prefer-optional-chain",
        summary: "Prefer optional chaining to equivalent guarded property access",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
