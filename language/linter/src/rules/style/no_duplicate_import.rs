use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow multiple import statements from one module.
    pub NO_DUPLICATE_IMPORT {
        id: "no-duplicate-import",
        summary: "Disallow multiple import statements from one module",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
