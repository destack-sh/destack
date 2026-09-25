use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow empty undecorated module declarations.
    pub NO_EMPTY_MODULE {
        id: "no-empty-module",
        summary: "Disallow empty undecorated module declarations",
        explanation: r#"
An undecorated module with no declarations or statements has no effect.
Instead, you SHOULD remove the empty module or add its intended contents.
"#,
        example: {
            reported: r#"
module {}
"#,
            accepted: r#"
module {
    export const version = 1;
}
"#,
        },
        provenance: [],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report empty module declarations without decorators or comments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect explicit module declarations
    for (declaration, node) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Module(declared) = node else {
            continue;
        };
        if !declared.expressions.is_empty() || view.has_decorators_any(declaration.into_any()) {
            continue;
        }

        // retain comments as an explicit reason for the empty body
        let span = module.source_extent(declaration.into_any())?;
        if module.has_unretained_comment(span, &[])? {
            continue;
        }
        output.report(lint.diagnostic("module is empty", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an empty module whose decorators configure compilation.
    #[test]
    fn test_accepts_decorated_module() {
        let session = TestSession::dir(
            &NO_EMPTY_MODULE,
            r#"
@noHeap
module {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a comment explaining an intentionally empty module.
    #[test]
    fn test_accepts_commented_module() {
        let session = TestSession::dir(
            &NO_EMPTY_MODULE,
            r#"
module {
    // populated by generated declarations
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
