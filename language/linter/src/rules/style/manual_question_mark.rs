use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer propagation over manually returning a failed Result.
    pub MANUAL_QUESTION_MARK {
        id: "manual-question-mark",
        summary: "Prefer propagation over manually returning a failed Result",
        explanation: r#"
Branching on a Result to select its successful payload or immediately return its error manually performs propagation.
Instead, you SHOULD use the question mark operator to propagate the failed Result.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = result?;
    return Result.ok(value);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result control flow that manually propagates its error.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect Result matches and error conditions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let propagation = match node {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    continue;
                };

                // recognize either ordering of successful extraction and error return
                let is_propagation = is_propagation(module, *first, *second)?
                    || is_propagation(module, *second, *first)?;

                is_propagation.then_some((*value, false))
            }
            dir::Expression::If { .. } => {
                conditional_propagation(module, expression)?.map(|value| (value, true))
            }
            _ => None,
        };
        let Some((value, is_statement)) = propagation else {
            continue;
        };
        if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Result) {
            continue;
        }
        let value_type = module.node_type_id(value.into_any())?;
        if module.dir.borrow_access(value_type)?.is_some() {
            continue;
        }

        // replace the complete control expression with propagation
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("control flow manually propagates a failed Result", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, value, is_statement)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether two arms extract success and return the unchanged error.
fn is_propagation(
    module: &DirModule<'_>,
    success_arm: dir::LocalNodeId<dir::MatchArm>,
    error_arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<bool, ProviderError> {
    // require the successful payload to be returned unchanged
    let Some((success_pattern, success_body)) = module.match_arm_value(success_arm) else {
        return Ok(false);
    };
    if module.pattern_language_item(success_pattern)? != Some(dir::LanguageItem::Ok) {
        return Ok(false);
    }
    if module
        .selected_pattern_binding(success_pattern, success_body)?
        .is_none()
    {
        return Ok(false);
    }

    // require the error payload to be returned through Result.err unchanged
    let Some((error_pattern, error_body)) = module.match_arm_expression(error_arm) else {
        return Ok(false);
    };

    returns_error(module, error_pattern, error_body)
}

/// Return one Result consumed by a failed-pattern condition and returned unchanged.
fn conditional_propagation(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();

    // select one unannotated failed-pattern condition without an else branch
    let dir::Expression::If {
        form: dir::IfForm::If,
        condition,
        then_expression,
        else_expression: None,
    } = view.get(expression)
    else {
        return Ok(None);
    };
    let Some((_, _, declarator)) = condition.as_binding() else {
        return Ok(None);
    };
    let declarator = view.get(declarator);
    if declarator.ty.is_some() {
        return Ok(None);
    }
    let Some(result) = declarator.value else {
        return Ok(None);
    };
    let Some(body) = module.sole_expression(*then_expression) else {
        return Ok(None);
    };
    if !returns_error(module, declarator.pattern, body)? {
        return Ok(None);
    }

    Ok(Some(result))
}

/// Return whether one failed Result pattern is returned unchanged.
fn returns_error(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // require one failed Result pattern returned through Result.err
    if module.pattern_language_item(pattern)? != Some(dir::LanguageItem::Err) {
        return Ok(false);
    }
    let dir::Expression::Return {
        value: Some(returned),
    } = view.get(body)
    else {
        return Ok(false);
    };
    let returned_type = module.adjusted_type_id(returned.into_any())?;
    if module.dir.representation_item(returned_type)? != Some(dir::LanguageItem::Result) {
        return Ok(false);
    }
    if module.language_member(*returned)? != Some(dir::LanguageItem::Result.member("err")) {
        return Ok(false);
    }

    // require the unchanged error as the sole argument
    let dir::Expression::Call { arguments, .. } = view.get(*returned) else {
        return Ok(false);
    };
    let [argument] = arguments.as_slice() else {
        return Ok(false);
    };
    let dir::Argument::Positional { value } = view.get(*argument) else {
        return Ok(false);
    };
    if module
        .coercions
        .coercion(value.into_global_any(module.id))
        .is_some()
    {
        return Ok(false);
    }

    Ok(module.selected_pattern_binding(pattern, *value)?.is_some())
}

/// Build one question mark propagation expression.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    result: dir::LocalNodeId<dir::Expression>,
    is_statement: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(result.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping before the postfix operator
    let result = module.expression_source(result, dir::OperatorPrecedence::Postfix)?;
    let terminator = if is_statement { ";" } else { "" };
    let patch = Patch::replace(extent, format!("{result}?{terminator}"));
    let suggestion = lint.fix("propagate the failed Result", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace manual error propagation.
    #[test]
    fn test_replaces_result_match() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            MANUAL_QUESTION_MARK.example.reported(),
        );

        session.assert_fixes(MANUAL_QUESTION_MARK.example.accepted());
    }

    /// Recognize reversed Result arm ordering.
    #[test]
    fn test_replaces_reversed_arms() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Err { error } => return Result.err(error)
        Ok { value } => value
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_fixes(MANUAL_QUESTION_MARK.example.accepted());
    }

    /// Replace propagation written in a sole-expression block.
    #[test]
    fn test_replaces_error_block() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => {
            return Result.err(error);
        }
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_fixes(MANUAL_QUESTION_MARK.example.accepted());
    }

    /// Replace a failed-pattern condition that returns its error unchanged.
    #[test]
    fn test_replaces_error_condition() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function consume(result: Result<int32, string>): Result<int32, string> {
    if (let Err { error } = result) {
        return Result.err(error);
    }
    return Result.ok(0);
}
"#,
        );

        session.assert_fixes(
            r#"
function consume(result: Result<int32, string>): Result<int32, string> {
    result?;
    return Result.ok(0);
}
"#,
        );
    }

    /// Accept an error transformation before returning.
    #[test]
    fn test_accepts_transformed_error() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(`invalid: ${error}`)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual propagation from a raw variant union without Try conformance.
    #[test]
    fn test_accepts_raw_variant_union() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: Ok<int32> | Err<string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve propagation from a borrowed Result that cannot satisfy Try by value.
    #[test]
    fn test_accepts_borrowed_result() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: &readonly Result<int32, int32>): Result<int32, int32> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve propagation into a return type that does not implement Try.
    #[test]
    fn test_accepts_union_return() {
        let session = TestSession::dir(
            &MANUAL_QUESTION_MARK,
            r#"
function value(result: Result<int32, string>): Result<int32, string> | undefined {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
