use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow useless regex backreferences.
    pub NO_USELESS_BACKREFERENCE {
        id: "no-useless-backreference",
        summary: "Disallow useless regex backreferences",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
