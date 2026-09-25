use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow parameterless lambdas called where they are defined.
    pub NO_REDUNDANT_CLOSURE_CALL {
        id: "no-redundant-closure-call",
        summary: "Disallow parameterless lambdas called where they are defined",
        explanation: r#"
Calling a parameterless lambda at its definition adds a callable boundary around one immediate computation.
Instead, you SHOULD evaluate the lambda body directly.
"#,
        example: {
            reported: r#"
function answer(): int32 {
    return (() => 42)();
}
"#,
            accepted: r#"
function answer(): int32 {
    return 42;
}
"#,
        },
        provenance: [Clippy("redundant_closure_call")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report parameterless synchronous lambdas called immediately.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct calls without arguments or optional evaluation
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            is_optional: false,
            ..
        } = node
        else {
            continue;
        };
        if !generic_arguments.is_empty() || !arguments.is_empty() {
            continue;
        }

        // require a parameterless synchronous lambda
        let Some(lambda) = module.lambda(*left) else {
            continue;
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync
            || lambda.signature.is_generator
            || lambda.signature.this_parameter.is_some()
            || !lambda.signature.parameters.is_empty()
            || !lambda.signature.generic_parameters.is_empty()
        {
            continue;
        }
        let Some(body) = lambda.body else {
            continue;
        };

        // report the complete immediate call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("lambda is called immediately", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *left, body)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one direct evaluation replacement when the lambda boundary is irrelevant.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    call: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let view = module.view();
    let mut extent = module.source_extent(call.into_any())?;
    if let Some(parentheses) = module.source_parentheses(callee.into_any()) {
        extent = extent.merge(parentheses);
    }
    let body_extent = module.source_extent(body.into_any())?;
    if module.has_unretained_comment(extent, &[body_extent])? {
        return Ok(None);
    }

    // retain expression bodies with the precedence required by the removed call
    let replacement = if !matches!(view.get(body), dir::Expression::Block(_)) {
        module
            .expression_source(body, dir::OperatorPrecedence::Postfix)?
            .into_owned()
    }
    // unwrap a block containing only one returned value
    else if let dir::Expression::Block(block) = view.get(body)
        && let Some(return_) = view.get(*block).only_expression()
        && let dir::Expression::Return { value: Some(value) } = view.get(return_)
    {
        let retained = module.source_extent((*value).into_any())?;
        if module.has_unretained_comment(body_extent, &[retained])? {
            return Ok(None);
        }

        module
            .expression_source(*value, dir::OperatorPrecedence::Postfix)?
            .into_owned()
    }
    // preserve a control-free statement block as a scoped do expression
    else if !module.uses_enclosing_control(body.into_any())? {
        format!("do {}", module.source(body_extent)?)
    } else {
        return Ok(None);
    };

    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("evaluate the lambda body directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve precedence when replacing a binary lambda body.
    #[test]
    fn test_preserves_binary_precedence() {
        let session = TestSession::dir(
            &NO_REDUNDANT_CLOSURE_CALL,
            r#"
function scale(value: int32): int32 {
    return 2 * (() => value + 1)();
}
"#,
        );

        session.assert_fixes(
            r#"
function scale(value: int32): int32 {
    return 2 * (value + 1);
}
"#,
        );
    }

    /// Replace a block lambda whose only statement returns a value.
    #[test]
    fn test_replaces_returning_block_lambda() {
        let session = TestSession::dir(
            &NO_REDUNDANT_CLOSURE_CALL,
            r#"
function answer(): int32 {
    return (() => {
        return 42;
    })();
}
"#,
        );

        session.assert_fixes(
            r#"
function answer(): int32 {
    return 42;
}
"#,
        );
    }

    /// Accept a lambda call whose argument binds a parameter.
    #[test]
    fn test_accepts_parameterized_lambda() {
        let session = TestSession::dir(
            &NO_REDUNDANT_CLOSURE_CALL,
            r#"
function answer(): int32 {
    return ((value: int32) => value)(42);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
