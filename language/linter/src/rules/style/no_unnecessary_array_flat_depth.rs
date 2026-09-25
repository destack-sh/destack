use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow an explicit default array flattening depth.
    pub NO_UNNECESSARY_ARRAY_FLAT_DEPTH {
        id: "no-unnecessary-array-flat-depth",
        summary: "Disallow an explicit default array flattening depth",
        explanation: r#"
`Array.flat(1)` requests the same one-level flattening performed by `Array.flat()`.
Instead, you SHOULD omit the redundant depth argument.
"#,
        example: {
            reported: r#"
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat(1);
}
"#,
            accepted: r#"
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat();
}
"#,
        },
        provenance: [Unicorn("no-unnecessary-array-flat-depth")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical Array flat calls with an explicit depth of one.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array flat calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("flat")) {
            continue;
        }

        // require one positional constant depth of one
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };
        if module.scalar_constant(*value)? != Some(dir::Literal::Integer(1)) {
            continue;
        }

        // remove the redundant argument without discarding comments
        let span = module.source_extent(value.into_any())?;
        let mut diagnostic = lint.diagnostic("flat call specifies its default depth", span);
        if let Some(suggestion) = suggest_omission(module, lint, expression)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build a zero-argument flat call.
fn suggest_omission(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve comments inside the argument container
    let arguments = module.source_region(expression.into_any(), NodeSpanRegion::Arguments)?;
    if module.has_unretained_comment(arguments, &[])? {
        return Ok(None);
    }

    // replace the complete argument container
    let patch = Patch::replace(arguments, "()");
    let suggestion = lint.fix("omit the default flattening depth", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a nondefault flattening depth.
    #[test]
    fn test_accepts_deeper_flattening() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_ARRAY_FLAT_DEPTH,
            r#"
function flatten(values: unknown[][][]): unknown[] {
    return values.flat(2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user method named flat.
    #[test]
    fn test_accepts_user_flat_method() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_ARRAY_FLAT_DEPTH,
            r#"
class Values {
    flat(depth: int32): this {
        return this;
    }
}

function flatten(values: Values): Values {
    return values.flat(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a comment beside the default depth by omitting the fix.
    #[test]
    fn test_reports_commented_depth_without_fix() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_ARRAY_FLAT_DEPTH,
            r#"
function flatten(values: (int32 | int32[])[]): void {
    values.flat(/* retain */ 1);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unnecessary-array-flat-depth]: flat call specifies its default depth
 ──▶ main.tspp:2:30
  │
1 │ function flatten(values: (int32 | int32[])[]): void {
2 │     values.flat(/* retain */ 1);
  │                              ^
3 │ }
  │
"#,
        );
    }
}
