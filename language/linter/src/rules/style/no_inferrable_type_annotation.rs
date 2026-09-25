use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow annotations identical to the type inferred from a scalar initializer.
    pub NO_INFERRABLE_TYPE_ANNOTATION {
        id: "no-inferrable-type-annotation",
        summary: "Disallow annotations identical to an inferred scalar type",
        explanation: r#"
A scalar initializer already determines its ordinary runtime type.
Instead, you SHOULD omit an annotation that repeats that type.

Annotations that preserve a literal type, select another width, or constrain a wider expression remain meaningful.
"#,
        example: {
            reported: r#"
const enabled: boolean = true;
"#,
            accepted: r#"
const enabled = true;
"#,
        },
        provenance: [TypeScriptEslint("no-inferrable-types")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report scalar annotations that exactly repeat initializer inference.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect variable declarators with initializers
    for (node, declarator) in view.iter_nodes::<dir::Declarator>() {
        if let (Some(annotation), Some(value)) = (declarator.ty, declarator.value) {
            report_redundant_annotation(
                module,
                lint,
                node.into_any(),
                annotation,
                value,
                &mut output,
            )?;
        }
    }

    // inspect initialized nominal fields
    for (node, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Field {
            declared_type: Some(annotation),
            default: Some(value),
            ..
        } = member
        else {
            continue;
        };
        report_redundant_annotation(
            module,
            lint,
            node.into_any(),
            *annotation,
            *value,
            &mut output,
        )?;
    }

    Ok(output)
}

/// Report one annotation when its initializer selects the same scalar representation.
fn report_redundant_annotation(
    module: &DirModule<'_>,
    lint: &Lint,
    owner: dir::LocalNodeIdAny,
    annotation: dir::LocalNodeId<dir::TypeExpression>,
    value: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    if !is_scalar_initializer(value, &module.view()) {
        return Ok(());
    }
    let annotation_type = module.node_type(annotation.into_any())?;
    let Some(inferred_type) = module.node_type(value.into_any())?.scalar_representation() else {
        return Ok(());
    };
    if annotation_type != inferred_type {
        return Ok(());
    }

    // identify the redundant type itself
    let span = module.source_extent(annotation.into_any())?;

    // remove the complete type annotation region
    let edit = module.source_region(owner, NodeSpanRegion::Type)?;
    let patch = Patch::delete(edit);
    let suggestion = lint.fix("remove the redundant type annotation", patch)?;
    let diagnostic = lint
        .diagnostic("type annotation repeats scalar inference", span)
        .suggestion(suggestion);
    output.report(diagnostic);

    Ok(())
}

/// Return whether one expression is a scalar literal with optional numeric sign.
fn is_scalar_initializer(value: dir::LocalNodeId<dir::Expression>, view: &dir::View<'_>) -> bool {
    if view.get(value).as_scalar().is_some() {
        return true;
    }

    matches!(
        view.get(value),
        dir::Expression::Unary {
            operator: dir::UnaryOperator::Plus | dir::UnaryOperator::Negate,
            right,
        } if view.get(*right).as_scalar().is_some_and(|literal| matches!(
            literal,
            dir::Literal::Integer(_) | dir::Literal::Float(_) | dir::Literal::Bigint(_)
        ))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an annotation that exactly repeats scalar inference.
    #[test]
    fn test_removes_inferrable_annotation() {
        let session = TestSession::dir(
            &NO_INFERRABLE_TYPE_ANNOTATION,
            r#"
const label: string = "one";
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-inferrable-type-annotation]: type annotation repeats scalar inference
 ──▶ main.tspp:1:14
  │
1 │ const label: string = "one";
  │              ^^^^^^
  │

 = fix: remove the redundant type annotation
--- a/main.tspp
+++ b/main.tspp

-   1│ const label: string = "one";
+   1│ const label = "one";
"#,
        );
        session.assert_fixes(
            r#"
const label = "one";
"#,
        );
    }

    /// Remove a matching annotation from an initialized field.
    #[test]
    fn test_removes_field_annotation() {
        let session = TestSession::dir(
            &NO_INFERRABLE_TYPE_ANNOTATION,
            r#"
class Options {
    enabled: boolean = true;
}
"#,
        );

        session.assert_fixes(
            r#"
class Options {
    enabled = true;
}
"#,
        );
    }

    /// Preserve annotations that change or narrow the inferred type.
    #[test]
    fn test_accepts_meaningful_annotations() {
        let session = TestSession::dir(
            &NO_INFERRABLE_TYPE_ANNOTATION,
            r#"
const exact: 1 = 1;
const narrow: int32 = 1;
const calculated: int32 = 1 + 2;

function choose(count: int64 = 1): void {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
