use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Discourage functions that only forward their parameters unchanged.
    pub REDUNDANT_FORWARDING_FUNCTION {
        id: "redundant-forwarding-function",
        summary: "Discourage functions that only forward their parameters unchanged",
        explanation: r#"
A function whose complete body calls another callable with the same parameters adds a name without adding behavior.
Instead, you SHOULD use the underlying callable directly.

Exported aliases, overrides, decorated functions, and ambient declarations remain intentional boundaries.
"#,
        example: {
            reported: r#"
declare function parse(source: string): int32;

function parseValue(source: string): int32 {
    return parse(source);
}
"#,
            accepted: r#"
declare function parse(source: string): int32;

const parseValue = parse;
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report private callables that only forward unchanged parameters.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect private free functions
    for (node, declaration) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Function(function) = declaration else {
            continue;
        };
        if function.export.is_some()
            || function.is_ambient
            || view.has_decorators_any(node.into_any())
            || !is_forwarding_signature(&function.signature, module)
        {
            continue;
        }
        let Some(body) = function.body else {
            continue;
        };
        report_forwarding(
            module,
            lint,
            node.into_any(),
            module.declaration_symbol(node)?,
            &function.signature.parameters,
            body,
            &mut output,
        )?;
    }

    // inspect explicitly private methods
    for (node, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            signature,
            body: Some(body),
            visibility: Some(dir::Visibility::Private),
            is_ambient: false,
            ..
        } = member
        else {
            continue;
        };
        if view.has_decorators_any(node.into_any()) || !is_forwarding_signature(signature, module) {
            continue;
        }
        report_forwarding(
            module,
            lint,
            node.into_any(),
            module.declaration_symbol(node)?,
            &signature.parameters,
            *body,
            &mut output,
        )?;
    }

    Ok(output)
}

/// Report one callable when its body only forwards its parameters.
fn report_forwarding(
    module: &DirModule<'_>,
    lint: &Lint,
    callable: dir::LocalNodeIdAny,
    callable_symbol: dir::GlobalSymbolId,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    body: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), destack_repository::ProviderError> {
    let Some(call) = forwarded_call(body, module) else {
        return Ok(());
    };
    let dir::Expression::Call {
        arguments,
        is_optional: false,
        ..
    } = module.view().get(call)
    else {
        return Ok(());
    };
    if arguments.len() != parameters.len() {
        return Ok(());
    }

    // require every argument to be its corresponding unadjusted parameter
    for (parameter, argument) in parameters.iter().zip(arguments) {
        let dir::Argument::Positional { value } = module.view().get(*argument) else {
            return Ok(());
        };
        let parameter = module.declaration_symbol(*parameter)?;
        if module.selected_symbol(*value)? != Some(parameter)
            || !module.is_unadjusted(value.into_any())
        {
            return Ok(());
        }
    }

    // reject direct recursion and report the forwarding declaration
    if module.call_symbol(call)? == Some(callable_symbol) {
        return Ok(());
    }
    let span = module.main_span(callable)?;
    output.report(lint.diagnostic("callable only forwards its parameters", span));

    Ok(())
}

/// Return whether one signature adds no call-entry behavior.
fn is_forwarding_signature(signature: &dir::FunctionSignature, module: &DirModule<'_>) -> bool {
    signature.asynchrony == dir::Asynchrony::Sync
        && !signature.is_generator
        && !signature.is_override
        && signature.role.is_none()
        && signature.parameters.iter().all(|parameter| {
            matches!(
                module.view().get(*parameter),
                dir::Parameter::Named {
                    default: None,
                    is_optional: false,
                    ..
                }
            )
        })
}

/// Return the sole call completed by one callable body.
fn forwarded_call(
    body: dir::LocalNodeId<dir::Expression>,
    module: &DirModule<'_>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let view = module.view();
    let expression = match view.get(body) {
        dir::Expression::Block(block) => view.get(*block).only_expression()?,
        _ => body,
    };
    let expression = match view.get(expression) {
        dir::Expression::Return { value: Some(value) } => *value,
        _ => expression,
    };

    matches!(view.get(expression), dir::Expression::Call { .. }).then_some(expression)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a private function that forwards every parameter unchanged.
    #[test]
    fn test_reports_forwarding_function() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function parse(source: string): int32;

function parseValue(source: string): int32 {
    return parse(source);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[redundant-forwarding-function]: callable only forwards its parameters
 ──▶ main.ds:3:10
  │
1 │ declare function parse(source: string): int32;
2 │
3 │ function parseValue(source: string): int32 {
  │          ^^^^^^^^^^
4 │     return parse(source);
5 │ }
  │
"#,
        );
    }

    /// Accept forwarding that changes an argument.
    #[test]
    fn test_accepts_transformed_argument() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function parse(source: string): int32;

function parseValue(source: string): int32 {
    return parse(source.trim());
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a forwarding function that supplies a default argument.
    #[test]
    fn test_accepts_defaulted_parameter() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function connect(timeout: int32): void;

function connectDefault(timeout: int32 = 30): void {
    connect(timeout);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an exported forwarding alias.
    #[test]
    fn test_accepts_exported_alias() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function parse(source: string): int32;

export function parseValue(source: string): int32 {
    return parse(source);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
