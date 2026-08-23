use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer widthless names for default-width scalar types.
    pub PREFER_SCALAR_ALIAS {
        id: "prefer-scalar-alias",
        summary: "Prefer widthless names for default-width scalar types",
        explanation: r#"
`int`, `uint`, and `float` name the default 64-bit scalar types, so `int64`, `uint64`, and `float64` write the width redundantly.
Instead, you SHOULD write the widthless name for a default-width scalar type.
"#,
        example: {
            reported: r#"
function scale(value: int64): int {
    return value * 2;
}
"#,
            accepted: r#"
function scale(value: int): int {
    return value * 2;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report default-width scalar types written with an explicit width.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // collect the type expressions standing as runtime values
    let mut value_positions = FxIndexSet::default();
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        if let dir::Expression::Type { value } = expression {
            value_positions.insert(*value);
        }
    }

    // inspect written scalar keywords in type position
    for (node, ty) in view.iter_nodes::<dir::TypeExpression>() {
        let dir::TypeExpression::Keyword { value } = ty else {
            continue;
        };
        let Some(alias) = widthless_alias(value) else {
            continue;
        };

        // report value positions without a fix: a binding could capture the widthless name
        if value_positions.contains(&node) {
            report_width_name(module, lint, &mut output, node.into_any(), alias, false)?;
            continue;
        }
        report_width_name(module, lint, &mut output, node.into_any(), alias, true)?;
    }

    // inspect written scalar names denoting types in expression position
    for (node, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Identifier { name } = expression else {
            continue;
        };
        let Some(alias) = dir::TypeLiteral::from_sized_name(module.dir.strings.get(*name))
            .as_ref()
            .and_then(widthless_alias)
        else {
            continue;
        };

        // require the name to denote the type, not a lexical binding
        let global = node.into_global_any(module.id);
        let denotes_type = module
            .resolutions
            .name_resolution(global)
            .is_some_and(|resolution| resolution.denoted_type().is_some());
        if !denotes_type {
            continue;
        }

        // report without a fix: a lexical binding could capture the widthless name here
        report_width_name(module, lint, &mut output, node.into_any(), alias, false)?;
    }

    Ok(output)
}

/// Return the widthless alias for one width-named default scalar.
fn widthless_alias(literal: &dir::TypeLiteral) -> Option<&'static str> {
    match literal {
        dir::TypeLiteral::Integer(dir::IntegerType::Fixed {
            width: 64,
            is_signed,
        }) => Some(if *is_signed { "int" } else { "uint" }),
        dir::TypeLiteral::Float(dir::FloatType::Float64) => Some("float"),
        _ => None,
    }
}

/// Report one width-named scalar with its widthless replacement.
///
/// A fixable position carries the machine patch; elsewhere a binding could
/// capture the widthless name, so the replacement stays a help line.
fn report_width_name(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
    node: dir::LocalNodeIdAny,
    alias: &str,
    is_fixable: bool,
) -> Result<(), ProviderError> {
    let span = module.source_extent(node)?;
    let diagnostic = lint.diagnostic("default-width scalar writes its width", span);
    let diagnostic = if is_fixable {
        diagnostic.suggestion(lint.fix(
            format!("write `{alias}`"),
            Patch::replace(span, alias.to_string()),
        )?)
    } else {
        diagnostic.help(format!("write `{alias}`"))
    };
    output.report(diagnostic);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an int64 written in type position.
    #[test]
    fn test_reports_written_int64() {
        TestSession::assert_example(&PREFER_SCALAR_ALIAS);
    }

    /// Report a float64 written in type position.
    #[test]
    fn test_reports_written_float64() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
function halve(value: float64): float {
    return value / 2.0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-scalar-alias]: default-width scalar writes its width
 ──▶ main.ds:1:23
  │
1 │ function halve(value: float64): float {
  │                       ^^^^^^^
2 │     return value / 2.0;
3 │ }
  │

 = fix: write `float`
--- a/main.ds
+++ b/main.ds

-   1│ function halve(value: float64): float {
+   1│ function halve(value: float): float {
"#,
        );
    }

    /// Report a uint64 static receiver in expression position.
    #[test]
    fn test_reports_uint64_static_receiver() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
const greatest = uint64.maximum();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-scalar-alias]: default-width scalar writes its width
 ──▶ main.ds:1:18
  │
1 │ const greatest = uint64.maximum();
  │                  ^^^^^^
  │

 = help: write `uint`
"#,
        );
    }

    /// Accept the widthless scalar names.
    #[test]
    fn test_accepts_widthless_names() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
function scale(value: int, factor: float): uint {
    return ((value as float) * factor) as uint;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep the report fixless when a binding shadows the widthless name.
    #[test]
    fn test_keeps_shadowed_alias_report_fixless() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
const uint = "shadow";
const greatest = uint64.maximum();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-scalar-alias]: default-width scalar writes its width
 ──▶ main.ds:2:18
  │
1 │ const uint = "shadow";
2 │ const greatest = uint64.maximum();
  │                  ^^^^^^
  │

 = help: write `uint`
"#,
        );
    }

    /// Keep a bare value-position report fixless.
    #[test]
    fn test_keeps_bare_value_report_fixless() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
function widths(): void {
    const meta = uint64;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-scalar-alias]: default-width scalar writes its width
 ──▶ main.ds:2:18
  │
1 │ function widths(): void {
2 │     const meta = uint64;
  │                  ^^^^^^
3 │ }
  │

 = help: write `uint`
"#,
        );
    }

    /// Accept the pointer-width scalar names.
    #[test]
    fn test_accepts_pointer_width_names() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
function measure(index: isize): usize {
    return index as usize;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept explicitly narrow scalar widths.
    #[test]
    fn test_accepts_narrow_widths() {
        let session = TestSession::dir(
            &PREFER_SCALAR_ALIAS,
            r#"
function narrow(value: int32): float32 {
    return value as float32;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
