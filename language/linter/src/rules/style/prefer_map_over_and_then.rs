use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Result map methods when chaining only rewraps one variant.
    pub PREFER_MAP_OVER_AND_THEN {
        id: "prefer-map-over-and-then",
        summary: "Prefer Result map methods when chaining only rewraps one variant",
        explanation: r#"
A Result chain that always rebuilds the same variant cannot introduce the opposite outcome.
Instead, you SHOULD use `map` or `mapErr` and return the transformed payload directly.
"#,
        example: {
            reported: r#"
function length(result: Result<string, int32>): Result<isize, int32> {
    return result.andThen((value) => Result.ok(value.length));
}
"#,
            accepted: r#"
function length(result: Result<string, int32>): Result<isize, int32> {
    return result.map((value) => value.length);
}
"#,
        },
        provenance: [Clippy("bind_instead_of_map")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One Result construction removed from a mapping callback.
#[derive(Debug, Clone, Copy)]
struct ResultConstruction {
    /// The complete constructor call.
    call: dir::LocalNodeId<dir::Expression>,
    /// The retained payload expression.
    value: dir::LocalNodeId<dir::Expression>,
}

/// Report Result chains whose callback always rebuilds the same variant.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Result chaining calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() {
            continue;
        }
        let (method, constructor) = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::Result.member("andThen") => {
                ("map", dir::LanguageItem::Result.member("ok"))
            }
            Some(member) if member == dir::LanguageItem::Result.member("orElse") => {
                ("mapErr", dir::LanguageItem::Result.member("err"))
            }
            _ => continue,
        };
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: callback } = view.get(*argument) else {
            continue;
        };

        // require one synchronous callback with a value on every path
        let Some(lambda) = module.lambda(*callback) else {
            continue;
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
            continue;
        }
        let Some(body) = lambda.body else {
            continue;
        };
        let Some(values) = module.callable_return_values(body) else {
            continue;
        };

        // require every callback result to use the same canonical constructor
        let mut constructions = Vec::with_capacity(values.len());
        for value in values {
            let Some(call) = module.member_call(value) else {
                constructions.clear();
                break;
            };
            if call.is_optional()
                || !call.generic_arguments.is_empty()
                || module.language_member(value)? != Some(constructor)
            {
                constructions.clear();
                break;
            }
            let [argument] = call.arguments else {
                constructions.clear();
                break;
            };
            let dir::Argument::Positional { value: payload } = view.get(*argument) else {
                constructions.clear();
                break;
            };

            constructions.push(ResultConstruction {
                call: value,
                value: *payload,
            });
        }
        if constructions.is_empty() {
            continue;
        }

        // replace chaining and unwrap every callback result
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("Result chain only rebuilds the selected variant", span);
        if call.generic_arguments.is_empty()
            && lambda.signature.return_type.is_none()
            && let Some(suggestion) = suggestion(module, lint, call.callee, method, &constructions)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one Result mapping rewrite.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    callee: dir::LocalNodeId<dir::Expression>,
    method: &str,
    constructions: &[ResultConstruction],
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let mut file = FilePatch::new(module.main_span(callee.into_any())?.file);

    // replace the chaining method
    let member = module.main_span(callee.into_any())?;
    file.replace(member, method);

    // replace each constructor with its retained payload
    for construction in constructions {
        let extent = module.source_extent(construction.call.into_any())?;
        let retained = module.source_extent(construction.value.into_any())?;
        if module.has_unretained_comment(extent, &[retained])? {
            return Ok(None);
        }
        let value =
            module.expression_source(construction.value, dir::OperatorPrecedence::Lowest)?;

        file.push(Patch::replace(extent, value.into_owned()));
    }

    file.sort();
    let suggestion = lint.fix("map the Result payload directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a failed Result chain.
    #[test]
    fn test_replaces_or_else() {
        let session = TestSession::dir(
            &PREFER_MAP_OVER_AND_THEN,
            r#"
function increment(result: Result<int32, int32>): Result<int32, int32> {
    return result.orElse((error) => Result.err(error + 1));
}
"#,
        );

        session.assert_fixes(
            r#"
function increment(result: Result<int32, int32>): Result<int32, int32> {
    return result.mapErr((error) => error + 1);
}
"#,
        );
    }

    /// Replace the same constructor across conditional callback paths.
    #[test]
    fn test_replaces_conditional_results() {
        let session = TestSession::dir(
            &PREFER_MAP_OVER_AND_THEN,
            r#"
function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.andThen((value) =>
        value < 0 ? Result.ok(-value) : Result.ok(value)
    );
}
"#,
        );

        session.assert_fixes(
            r#"
function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.map((value) =>
        value < 0 ? -value : value
    );
}
"#,
        );
    }

    /// Replace constructors in explicit callback returns.
    #[test]
    fn test_replaces_explicit_returns() {
        let session = TestSession::dir(
            &PREFER_MAP_OVER_AND_THEN,
            r#"
function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.andThen((value) => {
        if (value < 0) {
            return Result.ok(-value);
        }

        return Result.ok(value);
    });
}
"#,
        );

        session.assert_fixes(
            r#"
function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.map((value) => {
        if (value < 0) {
            return -value;
        }

        return value;
    });
}
"#,
        );
    }

    /// Accept a chain that can return an error.
    #[test]
    fn test_accepts_fallible_chain() {
        let session = TestSession::dir(
            &PREFER_MAP_OVER_AND_THEN,
            r#"
function parse(result: Result<string, string>): Result<int32, string> {
    return result.andThen((value) =>
        value.isEmpty ? Result.err("empty") : Result.ok(1)
    );
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without changing an explicit callback return type.
    #[test]
    fn test_preserves_callback_return_type() {
        let session = TestSession::dir(
            &PREFER_MAP_OVER_AND_THEN,
            r#"
function forward(result: Result<int32, string>): Result<int32, string> {
    return result.andThen((value): Result<int32, string> => Result.ok(value));
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-map-over-and-then]: Result chain only rebuilds the selected variant
 ──▶ main.tspp:2:12
  │
1 │ function forward(result: Result<int32, string>): Result<int32, string> {
2 │     return result.andThen((value): Result<int32, string> => Result.ok(value));
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
