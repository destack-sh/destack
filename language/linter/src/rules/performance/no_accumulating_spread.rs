use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow repeated spreading into an accumulating value.
    pub NO_ACCUMULATING_SPREAD {
        id: "no-accumulating-spread",
        summary: "Disallow repeated spreading into an accumulating value",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
