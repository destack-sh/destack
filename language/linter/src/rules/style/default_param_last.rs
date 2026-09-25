use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require required parameters before optional and defaulted parameters.
    pub DEFAULT_PARAM_LAST {
        id: "default-param-last",
        summary: "Require optional and defaulted parameters after required parameters",
        explanation: r#"
A required parameter after an optional or defaulted parameter prevents callers from omitting the earlier argument position.
Instead, you SHOULD place every required parameter before optional and defaulted parameters.
"#,
        example: {
            reported: r#"
function connect(timeout: int32 = 30, retries: int32): void {}
"#,
            accepted: r#"
function connect(retries: int32, timeout: int32 = 30): void {}
"#,
        },
        provenance: [
            Eslint("default-param-last"),
            TypeScriptEslint("default-param-last"),
        ],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report required parameters that follow optional or defaulted parameters.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored callable signature
    for node in view.iter_node_ids() {
        let Some(parameters) = module.callable_parameters(node) else {
            continue;
        };

        report_parameters(module, lint, parameters, &mut output)?;
    }

    Ok(output)
}

/// Report required parameters after the first optional or defaulted parameter.
fn report_parameters(
    module: &DirModule<'_>,
    lint: &Lint,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();
    let mut has_optional = false;

    // preserve the signature's authored parameter order
    for parameter in parameters {
        let value = view.get(*parameter);
        let is_optional = value.is_optional() || value.default_value().is_some();
        let is_variadic = matches!(
            value,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );

        // report required parameters after the optional suffix begins
        if has_optional && !is_optional && !is_variadic {
            let span = module.main_span(parameter.into_any())?;
            output.report(lint.diagnostic(
                "required parameter follows an optional or defaulted parameter",
                span,
            ));
        }

        // retain whether an optional position has been crossed
        has_optional |= is_optional;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a required parameter after an optional method parameter.
    #[test]
    fn test_reports_required_method_parameter_after_optional() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
interface Client {
    connect(timeout?: int32, retries: int32): void;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:2:30
  │
1 │ interface Client {
2 │     connect(timeout?: int32, retries: int32): void;
  │                              ^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report a required class method parameter after one defaulted parameter.
    #[test]
    fn test_reports_required_class_method_parameter_after_default() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
class Client {
    connect(timeout: int32 = 30, retries: int32): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:2:34
  │
1 │ class Client {
2 │     connect(timeout: int32 = 30, retries: int32): void {}
  │                                  ^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report a required object method parameter after one defaulted parameter.
    #[test]
    fn test_reports_required_object_method_parameter_after_default() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
const client = {
    connect(timeout: int32 = 30, retries: int32): void {},
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:2:34
  │
1 │ const client = {
2 │     connect(timeout: int32 = 30, retries: int32): void {},
  │                                  ^^^^^^^
3 │ };
  │
"#,
        );
    }

    /// Report a required call-signature parameter after one optional parameter.
    #[test]
    fn test_reports_required_call_parameter_after_optional() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
interface Callable {
    (timeout?: int32, retries: int32): void;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:2:23
  │
1 │ interface Callable {
2 │     (timeout?: int32, retries: int32): void;
  │                       ^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report a required construct-signature parameter after one optional parameter.
    #[test]
    fn test_reports_required_construct_parameter_after_optional() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
class Client {}
interface Constructor {
    new (timeout?: int32, retries: int32): Client;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:3:27
  │
1 │ class Client {}
2 │ interface Constructor {
3 │     new (timeout?: int32, retries: int32): Client;
  │                           ^^^^^^^
4 │ }
  │
"#,
        );
    }

    /// Report a required function-type parameter after one optional parameter.
    #[test]
    fn test_reports_required_function_type_parameter_after_optional() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
type Callable = (timeout?: int32, retries: int32) => void;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:1:35
  │
1 │ type Callable = (timeout?: int32, retries: int32) => void;
  │                                   ^^^^^^^
  │
"#,
        );
    }

    /// Report a required constructor-type parameter after one optional parameter.
    #[test]
    fn test_reports_required_constructor_type_parameter_after_optional() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
class Client {}
type Constructor = new (timeout?: int32, retries: int32) => Client;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-param-last]: required parameter follows an optional or defaulted parameter
 ──▶ main.tspp:2:42
  │
1 │ class Client {}
2 │ type Constructor = new (timeout?: int32, retries: int32) => Client;
  │                                          ^^^^^^^
  │
"#,
        );
    }

    /// Accept a variadic parameter after one defaulted parameter.
    #[test]
    fn test_accepts_variadic_parameter_after_default() {
        let session = TestSession::dir(
            &DEFAULT_PARAM_LAST,
            r#"
function format(prefix: string = "", ...values: string[]): string {
    return prefix;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
