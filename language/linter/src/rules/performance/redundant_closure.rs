use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

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

const operation: (value: int32) => int32 = (value) => square(value);
"#,
            accepted: r#"
declare function square(value: int32): int32;

const operation: (value: int32) => int32 = square;
"#,
        },
        provenance: [Clippy("redundant_closure")],
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
        let Some(callee) = select_forwarded_callee(module, expression)? else {
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

/// Select the stable function called by one exact forwarding lambda.
fn select_forwarded_callee(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(lambda) = module.lambda(expression) else {
        return Ok(None);
    };
    let Some(body) = lambda.body else {
        return Ok(None);
    };
    let Some(call) = module.parameter_forwarding_call(&lambda.signature, body)? else {
        return Ok(None);
    };
    let dir::Expression::Call { left: callee, .. } = module.view().get(call) else {
        return Ok(None);
    };

    // require the function to take the parameter list the closure's slot declares
    let slot = module.adjusted_type_id(expression.into_any())?;
    let slot_parameters = module.signature_parameter_count(slot)?;
    let callee_parameters = module.callable_parameter_count(*callee)?;
    if slot_parameters != callee_parameters {
        return Ok(None);
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

    /// Accept a closure that discards the forwarded call's result.
    #[test]
    fn test_accepts_discarded_result() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
declare function inspect(value: int32): int32;

const inspectValue: (value: int32) => void = (value) => {
    inspect(value);
};
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a closure that constructs a newtype from its parameter.
    #[test]
    fn test_accepts_newtype_construction() {
        let session = TestSession::dir(
            &REDUNDANT_CLOSURE,
            r#"
newtype EntityRef = int32;

function wrap(result: Result<int32, string>): Result<EntityRef, string> {
    return result.map((id) => EntityRef(id));
}
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
