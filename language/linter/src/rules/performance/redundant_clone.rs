use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow clones proven unnecessary by ownership and liveness.
    pub REDUNDANT_CLONE {
        id: "redundant-clone",
        summary: "Disallow clones proven unnecessary by ownership and liveness",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
