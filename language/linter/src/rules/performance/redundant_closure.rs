use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow closures that only forward their arguments to another function.
    pub REDUNDANT_CLOSURE {
        id: "redundant-closure",
        summary: "Disallow closures that only forward their arguments to another function",
        explanation: r#"
A closure that passes each parameter unchanged to one function adds another callable value and invocation without changing the call.
Instead, you SHOULD pass the function directly when the forwarded arguments require no adjustments.
"#,
        example: {
            reported: r#"
declare function square(value: int32): int32;

function squares(values: int32[]): Iterator<int32> {
    return values.iterator().map((value) => square(value));
}
"#,
            accepted: r#"
declare function square(value: int32): int32;

function squares(values: int32[]): Iterator<int32> {
    return values.iterator().map(square);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report forwarding lambdas that can be replaced by their function.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect authored lambda expressions
    for (expression, _) in module.view().iter_nodes::<dir::Expression>() {
        let Some(callee) = forwarded_callee(module, expression)? else {
            continue;
        };

        // replace the complete lambda with the stable function
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("closure only forwards its parameters", span);
        if let Some(suggestion) = suggestion(module, lint, expression, callee)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the stable function called by one exact forwarding lambda.
fn forwarded_callee(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(lambda) = module.lambda(expression) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let Some(body) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(None);
    };
    let dir::Expression::Call {
        left: callee,
        generic_arguments,
        arguments,
        is_optional: false,
        ..
    } = module.view().get(body)
    else {
        return Ok(None);
    };
    if !generic_arguments.is_empty() || arguments.len() != lambda.signature.parameters.len() {
        return Ok(None);
    }
    if lambda.signature.parameters.iter().any(|parameter| {
        !matches!(
            module.view().get(*parameter),
            dir::Parameter::Named { default: None, .. }
        )
    }) {
        return Ok(None);
    }

    // require the selected function to accept exactly the forwarded arity
    let Some(parameters) = module.call_parameters(body)? else {
        return Ok(None);
    };
    if parameters.len() != arguments.len() || parameters.iter().any(|parameter| parameter.is_rest) {
        return Ok(None);
    }
    if module
        .coercions
        .coercion(body.into_global_any(module.id))
        .is_some()
    {
        return Ok(None);
    }

    // require a stable declaration function rather than a callable value or method
    if !matches!(
        module.view().get(*callee),
        dir::Expression::Identifier { .. }
    ) {
        return Ok(None);
    }
    let Some(function) = module.call_symbol(body)? else {
        return Ok(None);
    };
    if module.dir.has_decorators(function)? {
        return Ok(None);
    }

    // require every positional argument to be its corresponding unadjusted parameter
    for (parameter, argument) in lambda.signature.parameters.iter().zip(arguments) {
        let dir::Argument::Positional { value } = module.view().get(*argument) else {
            return Ok(None);
        };
        let parameter = module.declaration_symbol(*parameter)?;
        if module.selected_symbol(*value)? != Some(parameter)
            || module
                .coercions
                .coercion(value.into_global_any(module.id))
                .is_some()
        {
            return Ok(None);
        }
    }

    Ok(Some(*callee))
}

/// Replace one forwarding lambda with its called function.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let callee_span = module.source_extent(callee.into_any())?;
    if module.has_unretained_comment(span, &[callee_span])? {
        return Ok(None);
    }

    let callee = module.source(callee_span)?;
    let suggestion = lint.fix("pass the function directly", Patch::replace(span, callee))?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Replace a standalone forwarding closure with its function.
    #[test]
    fn test_replaces_standalone_forwarding_closure() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function square(value: int32): int32;

const operation: (value: int32) => int32 = (value) => square(value);
"#,
        );

        session.assert_fixes(
            r#"
declare function square(value: int32): int32;

const operation: (value: int32) => int32 = square;
"#,
        );
    }

    /// Accept a function with an omitted optional parameter.
    #[test]
    fn test_accepts_omitted_optional_parameter() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function parse(value: string, radix?: isize): isize;

const operation: (value: string, index: isize) => isize = (value) => parse(value);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a closure whose parameter supplies a default before forwarding.
    #[test]
    fn test_accepts_defaulted_parameter() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function square(value: int32): int32;

const operation = (value: int32 = 2) => square(value);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a rest function that could observe additional callback arguments.
    #[test]
    fn test_accepts_rest_function() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function first(...values: int32[]): int32;

const operation: (value: int32, index: isize) => int32 = (value) => first(value);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept forwarding through a callable value whose identity can change.
    #[test]
    fn test_accepts_callable_value() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
let operation = (value: int32) => value * value;

const forwarded = (value: int32) => operation(value);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept forwarding an unsafe function whose call site must remain explicit.
    #[test]
    fn test_accepts_unsafe_function() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
/// # Safety
///
/// The caller must validate `value`.
@unsafe
declare function unchecked(value: int32): int32;

const operation = (value: int32) => unchecked(value);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a closure that changes its argument.
    #[test]
    fn test_accepts_transformed_argument() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function square(value: int32): int32;

function squares(values: int32[]): Iterator<int32> {
    return values.iterator().map((value) => square(value + 1));
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a closure that performs work before forwarding.
    #[test]
    fn test_accepts_preceding_work() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function square(value: int32): int32;

const operation = (value: int32) => {
    value;

    return square(value);
};
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a closure that changes argument order.
    #[test]
    fn test_accepts_reordered_arguments() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function subtract(left: int32, right: int32): int32;

const difference = (left: int32, right: int32) => subtract(right, left);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a method call because extracting it would lose its receiver.
    #[test]
    fn test_accepts_forwarded_method() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
class Counter {
    add(value: int32): int32 {
        return value;
    }
}

function add(counter: Counter, values: int32[]): Iterator<int32> {
    return values.iterator().map((value) => counter.add(value));
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
