use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow empty functions.
    pub NO_EMPTY_FUNCTION {
        id: "no-empty-function",
        summary: "Disallow empty functions",
        explanation: r#"
An uncommented empty concrete function and an accidentally unfinished function have the same source shape.
Instead, you SHOULD remove the function, implement it, or explain the intentional empty body with a comment.
"#,
        example: {
            reported: r#"
function initialize(): void {}
"#,
            accepted: r#"
function initialize(): void {
    // initialization is intentionally unnecessary
}
"#,
        },
        provenance: [
            Eslint("no-empty-function"),
            TypeScriptEslint("no-empty-function"),
        ],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report concrete function bodies without expressions or comments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect declared functions
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Function(function) = declaration else {
            continue;
        };
        let Some(body) = function.body else {
            continue;
        };
        report_empty_body(module, lint, body, &mut output)?;
    }

    // inspect declaration members
    for (_, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            body: Some(body), ..
        } = member
        else {
            continue;
        };
        report_empty_body(module, lint, *body, &mut output)?;
    }

    // inspect object members
    for (_, property) in view.iter_nodes::<dir::Property>() {
        let dir::Property::Method {
            body: Some(body), ..
        } = property
        else {
            continue;
        };
        report_empty_body(module, lint, *body, &mut output)?;
    }

    // inspect nominal interface methods
    for (_, member) in view.iter_nodes::<dir::TypeMember>() {
        let dir::TypeMember::Method {
            body: Some(body), ..
        } = member
        else {
            continue;
        };
        report_empty_body(module, lint, *body, &mut output)?;
    }

    Ok(output)
}

/// Report one empty concrete body.
fn report_empty_body(
    module: &DirModule<'_>,
    lint: &Lint,
    body: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // require an empty block body without retained comments
    let dir::Expression::Block(block) = view.get(body) else {
        return Ok(());
    };
    if !view.get(*block).is_empty() {
        return Ok(());
    }
    let span = module.source_extent(body.into_any())?;
    if module.has_unretained_comment(span, &[])? {
        return Ok(());
    }

    // report the complete empty body
    output.report(lint.diagnostic("function body is empty", span));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an ambient function declaration without a body.
    #[test]
    fn test_accepts_ambient_function() {
        let session = TestSession::dir(
            &NO_EMPTY_FUNCTION,
            r#"
declare function initialize(): void;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an empty concrete method body.
    #[test]
    fn test_reports_empty_method() {
        let session = TestSession::dir(
            &NO_EMPTY_FUNCTION,
            r#"
class Service {
    initialize(): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-function]: function body is empty
 ──▶ main.tspp:2:24
  │
1 │ class Service {
2 │     initialize(): void {}
  │                        ^^
3 │ }
  │
"#,
        );
    }

    /// Report an empty default method on a nominal interface.
    #[test]
    fn test_reports_empty_interface_method() {
        let session = TestSession::dir(
            &NO_EMPTY_FUNCTION,
            r#"
newtype interface Service {
    initialize(): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-function]: function body is empty
 ──▶ main.tspp:2:24
  │
1 │ newtype interface Service {
2 │     initialize(): void {}
  │                        ^^
3 │ }
  │
"#,
        );
    }

    /// Report an empty object method body.
    #[test]
    fn test_reports_empty_object_method() {
        let session = TestSession::dir(
            &NO_EMPTY_FUNCTION,
            r#"
const service = {
    initialize(): void {},
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-function]: function body is empty
 ──▶ main.tspp:2:24
  │
1 │ const service = {
2 │     initialize(): void {},
  │                        ^^
3 │ };
  │
"#,
        );
    }

    /// Report an empty anonymous function body.
    #[test]
    fn test_reports_empty_lambda() {
        let session = TestSession::dir(
            &NO_EMPTY_FUNCTION,
            r#"
const initialize: () => void = (): void => {};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-function]: function body is empty
 ──▶ main.tspp:1:44
  │
1 │ const initialize: () => void = (): void => {};
  │                                            ^^
  │
"#,
        );
    }
}
