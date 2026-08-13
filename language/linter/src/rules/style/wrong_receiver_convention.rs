use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require receiver forms matching the method's name convention.
    pub WRONG_RECEIVER_CONVENTION {
        id: "wrong-receiver-convention",
        summary: "Require receiver forms matching the method's name convention",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
