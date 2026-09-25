use tspp_dir as dir;

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
        provenance: [],
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
            || function.signature.form != dir::FunctionForm::Function
            || view.has_decorators_any(node.into_any())
            || function.signature.is_override
            || function.signature.role.is_some()
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
            &function.signature,
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
        if view.has_decorators_any(node.into_any())
            || signature.is_override
            || signature.role.is_some()
        {
            continue;
        }
        report_forwarding(
            module,
            lint,
            node.into_any(),
            module.declaration_symbol(node)?,
            signature,
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
    signature: &dir::FunctionSignature,
    body: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), tspp_repository::ProviderError> {
    let Some(call) = module.parameter_forwarding_call(signature, body)? else {
        return Ok(());
    };

    // reject direct recursion and report the forwarding declaration
    if module.call_symbol(call)? == Some(callable_symbol) {
        return Ok(());
    }
    let span = module.main_span(callable)?;
    output.report(lint.diagnostic("callable only forwards its parameters", span));

    Ok(())
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
 ──▶ main.tspp:3:10
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

    /// Accept a function that discards the forwarded call's result.
    #[test]
    fn test_accepts_discarded_result() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function inspect(value: int32): int32;

function inspectValue(value: int32): void {
    inspect(value);
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

    /// Accept forwarding that applies a return adjustment or generic arguments.
    #[test]
    fn test_accepts_adjusted_forwarding() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
declare function read(): int32;
declare function identity<T>(value: T): T;

function readValue(): int32 | string {
    return read();
}

function text(value: string): string {
    return identity<string>(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept lambdas because redundant closures govern callable values.
    #[test]
    fn test_accepts_lambda() {
        let session = TestSession::dir(
            &REDUNDANT_FORWARDING_FUNCTION,
            r#"
newtype EntityRef = int32;

const wrap = (id: int32) => EntityRef(id);
"#,
        );

        session.assert_no_diagnostics();
    }
}
