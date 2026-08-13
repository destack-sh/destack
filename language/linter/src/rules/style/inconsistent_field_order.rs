use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require literal fields in declaration order.
    pub INCONSISTENT_FIELD_ORDER {
        id: "inconsistent-field-order",
        summary: "Require literal fields in declaration order",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
