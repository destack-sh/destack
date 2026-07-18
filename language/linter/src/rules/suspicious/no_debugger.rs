use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};
use destack_dir as dir;

declare_lint! {
    /// Disallow debugger statements.
    pub NO_DEBUGGER {
        id: "no-debugger",
        code: "LU008",
        description: "Disallow debugger statements",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-debugger.
fn check(mut context: DirModuleContext<'_>) -> Result<(), LinterError> {
    let view = context.module.view();

    // report every visible debugger expression in the checked DIR
    for (node, expression) in view.iter_nodes_of_type::<dir::Expression>() {
        if !matches!(expression, dir::Expression::Debugger) {
            continue;
        }

        let anchor = context.module.anchor(node.into_any());
        let diagnostic = context
            .diagnostic("debugger statement is not allowed", anchor)
            .label("remove this debugger statement");
        context.report(diagnostic);
    }

    Ok(())
}
