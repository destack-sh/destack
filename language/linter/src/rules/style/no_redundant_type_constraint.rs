use tspp_dir as dir;
use tspp_source::{NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow generic constraints equal to the implicit top type.
    pub NO_REDUNDANT_TYPE_CONSTRAINT {
        id: "no-redundant-type-constraint",
        summary: "Disallow generic constraints equal to the implicit top type",
        explanation: r#"
An `unknown` generic constraint accepts the same arguments as an unconstrained type parameter.
Instead, you SHOULD omit the redundant constraint.
"#,
        example: {
            reported: r#"
function identity<T: unknown>(value: T): T {
    return value;
}
"#,
            accepted: r#"
function identity<T>(value: T): T {
    return value;
}
"#,
        },
        provenance: [TypeScriptEslint("no-unnecessary-type-constraint")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report explicit top-type constraints on generic parameters.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect written type and variadic type parameters
    for (parameter, value) in view.iter_nodes::<dir::GenericParameter>() {
        let constraint = match value {
            dir::GenericParameter::Type {
                constraint: Some(constraint),
                ..
            }
            | dir::GenericParameter::VariadicType {
                constraint: Some(constraint),
                ..
            } => *constraint,
            _ => continue,
        };
        let is_top = matches!(
            view.get(constraint),
            dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::Unknown
            }
        );
        if !is_top {
            continue;
        }

        // identify the redundant constraint itself
        let span = module.source_extent(constraint.into_any())?;

        // remove its adjoining constraint syntax
        let edit = module.source_region(parameter.into_any(), NodeSpanRegion::Type)?;
        let patch = Patch::delete(edit);
        let suggestion = lint.fix("remove the redundant constraint", patch)?;
        let diagnostic = lint
            .diagnostic("generic constraint is implied", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an explicit unknown constraint.
    #[test]
    fn test_removes_unknown_constraint() {
        let session = TestSession::dir(
            &NO_REDUNDANT_TYPE_CONSTRAINT,
            r#"
function identity<T: unknown>(value: T): T {
    return value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-redundant-type-constraint]: generic constraint is implied
 ──▶ main.tspp:1:22
  │
1 │ function identity<T: unknown>(value: T): T {
  │                      ^^^^^^^
2 │     return value;
3 │ }
  │

 = fix: remove the redundant constraint
--- a/main.tspp
+++ b/main.tspp

-   1│ function identity<T: unknown>(value: T): T {
+   1│ function identity<T>(value: T): T {
    2│     return value;
"#,
        );
        session.assert_fixes(
            r#"
function identity<T>(value: T): T {
    return value;
}
"#,
        );
    }

    /// Remove a redundant constraint from a declaration generic.
    #[test]
    fn test_removes_class_constraint() {
        let session = TestSession::dir(
            &NO_REDUNDANT_TYPE_CONSTRAINT,
            r#"
class Box<T: unknown> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
"#,
        );
    }

    /// Preserve a constraint that narrows accepted arguments.
    #[test]
    fn test_accepts_narrowing_constraint() {
        let session = TestSession::dir(
            &NO_REDUNDANT_TYPE_CONSTRAINT,
            r#"
function widen<T: number>(value: T): number {
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
