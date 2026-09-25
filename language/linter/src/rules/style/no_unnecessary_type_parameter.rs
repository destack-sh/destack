use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow type parameters whose constraints can replace them without changing behavior.
    pub NO_UNNECESSARY_TYPE_PARAMETER {
        id: "no-unnecessary-type-parameter",
        summary: "Disallow type parameters that add no relationship or precision",
        explanation: r#"
A callable type parameter used once in its signature relates no input or output types.
Instead, you SHOULD use its constraint directly.

A type parameter remains useful when it appears in multiple signature positions or participates in another generic relation.
"#,
        example: {
            reported: r#"
function printValue<T: Display>(value: &immutable T): void {
    value.display();
}
"#,
            accepted: r#"
function choose<T: Display>(left: T, right: T): T {
    return left;
}
"#,
        },
        provenance: [TypeScriptEslint("no-unnecessary-type-parameters")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report constrained callable type parameters used once in their signature.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored callable signature
    for node in view.iter_node_ids() {
        let Some(parameters) = module.callable_generic_parameters(node) else {
            continue;
        };
        check_parameters(
            module,
            lint,
            node,
            parameters,
            module.callable_body(node),
            &mut output,
        )?;
    }

    Ok(output)
}

/// Report unnecessary parameters from one callable signature.
fn check_parameters(
    module: &DirModule<'_>,
    lint: &Lint,
    callable: dir::LocalNodeIdAny,
    parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    body: Option<dir::LocalNodeId<dir::Expression>>,
    output: &mut LintOutput,
) -> Result<(), tspp_repository::ProviderError> {
    let view = module.view();

    // require one ordinary constrained parameter without other generic behavior
    for parameter in parameters {
        let dir::GenericParameter::Type {
            variance: None,
            constraint: Some(_),
            default: None,
            is_const: false,
            ..
        } = view.get(*parameter)
        else {
            continue;
        };
        let symbol = module.declaration_symbol(*parameter)?;
        let signature_uses = module
            .symbol_references(symbol)
            .filter(|reference| view.is_inside(*reference, callable))
            .filter(|reference| {
                body.is_none_or(|body| !view.is_inside(*reference, body.into_any()))
            })
            .count();
        if signature_uses != 1 {
            continue;
        }

        let span = module.main_span(parameter.into_any())?;
        output
            .report(lint.diagnostic("type parameter occurs once in the callable signature", span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a constrained type parameter used in one signature position.
    #[test]
    fn test_reports_single_signature_use() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TYPE_PARAMETER,
            r#"
function display<T: Display>(value: &immutable T): void {
    value.display();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unnecessary-type-parameter]: type parameter occurs once in the callable signature
 ──▶ main.tspp:1:18
  │
1 │ function display<T: Display>(value: &immutable T): void {
  │                  ^
2 │     value.display();
3 │ }
  │
"#,
        );
    }

    /// Accept a parameter relating multiple signature positions.
    #[test]
    fn test_accepts_related_signature_types() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TYPE_PARAMETER,
            r#"
function choose<T: Display>(left: T, right: T): T {
    return left;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a constrained parameter used once in a structural call signature.
    #[test]
    fn test_reports_structural_call_signature() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TYPE_PARAMETER,
            r#"
interface Formatter {
    <T: Display>(value: &immutable T): void;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unnecessary-type-parameter]: type parameter occurs once in the callable signature
 ──▶ main.tspp:2:6
  │
1 │ interface Formatter {
2 │     <T: Display>(value: &immutable T): void;
  │      ^
3 │ }
  │
"#,
        );
    }
}
