use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require protocol implementation for members named like protocol operations.
    pub PREFER_PROTOCOL_IMPLEMENTATION {
        id: "prefer-protocol-implementation",
        summary: "Require protocol implementation for members named like protocol operations",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
