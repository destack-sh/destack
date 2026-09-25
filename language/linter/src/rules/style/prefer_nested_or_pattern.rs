use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer nesting or-patterns inside their shared nominal pattern.
    pub PREFER_NESTED_OR_PATTERN {
        id: "prefer-nested-or-pattern",
        summary: "Prefer nesting or-patterns inside their shared nominal pattern",
        explanation: r#"
Repeating the same nominal pattern in every or-pattern branch obscures the field that varies.
Instead, you SHOULD place the alternatives in the shared field pattern.
"#,
        example: {
            reported: r#"
function isSmall(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value: 0 } | Ok { value: 1 } => true
        _ => false
    };
}
"#,
            accepted: r#"
function isSmall(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value: 0 | 1 } => true
        _ => false
    };
}
"#,
        },
        provenance: [Clippy("unnested_or_patterns")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report or-patterns that repeat one nominal pattern and field.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored unions with at least two alternatives
    for (pattern, node) in view.iter_nodes::<dir::Pattern>() {
        let dir::Pattern::Union { patterns } = node else {
            continue;
        };
        if patterns.len() < 2 {
            continue;
        }

        // require one identical nominal projection around each nested pattern
        let mut selection = None;
        let mut prefix = None;
        let mut suffix = None;
        let mut nested_sources = Vec::with_capacity(patterns.len());
        let mut retained = Vec::with_capacity(patterns.len() + 2);
        let mut is_shared = true;
        for alternative in patterns {
            let alternative_selection = match module.pattern_decision(*alternative)? {
                dir::PatternDecision::Destructure(destructure) => match destructure.as_ref() {
                    dir::PatternDestructureResolution::Nominal(nominal) => &nominal.key,
                    _ => {
                        is_shared = false;
                        break;
                    }
                },
                dir::PatternDecision::Project(project) => match &project.projection {
                    dir::OperationResolution::One(dir::Projection::NewtypePayload {
                        key, ..
                    }) => key,
                    _ => {
                        is_shared = false;
                        break;
                    }
                },
                _ => {
                    is_shared = false;
                    break;
                }
            };
            if selection
                .as_ref()
                .is_some_and(|selection| *selection != alternative_selection)
            {
                is_shared = false;
                break;
            }
            selection = Some(alternative_selection);

            // select the sole explicitly nested field
            let field = match view.get(*alternative) {
                dir::Pattern::NominalTuple { fields, .. }
                | dir::Pattern::NominalObject { fields, .. }
                    if fields.len() == 1 =>
                {
                    fields[0]
                }
                _ => {
                    is_shared = false;
                    break;
                }
            };
            let nested = match view.get(field) {
                dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                }
                | dir::PatternField::Computed { pattern, .. }
                | dir::PatternField::Positional { pattern } => *pattern,
                _ => {
                    is_shared = false;
                    break;
                }
            };

            // require identical authored text around the varying child
            let alternative_extent = module.source_extent(alternative.into_any())?;
            let nested_extent = module.source_extent(nested.into_any())?;
            let alternative_prefix = module.source(tspp_source::Span::new(
                alternative_extent.file,
                alternative_extent.start,
                nested_extent.start,
            ))?;
            let alternative_suffix = module.source(tspp_source::Span::new(
                alternative_extent.file,
                nested_extent.end,
                alternative_extent.end,
            ))?;
            if prefix.is_some_and(|prefix| prefix != alternative_prefix)
                || suffix.is_some_and(|suffix| suffix != alternative_suffix)
            {
                is_shared = false;
                break;
            }
            prefix = Some(alternative_prefix);
            suffix = Some(alternative_suffix);
            nested_sources.push(module.source(nested_extent)?);
            retained.push(nested_extent);
        }
        if !is_shared {
            continue;
        }

        // replace the repeated wrappers when their comments remain represented
        let extent = module.source_extent(pattern.into_any())?;
        let mut diagnostic = lint.diagnostic("or-pattern repeats the same nominal pattern", extent);
        let first_extent = module.source_extent(patterns[0].into_any())?;
        let Some(first_nested) = retained.first().copied() else {
            return Err(ProviderError::internal(
                "shared nominal or-pattern has no retained nested pattern",
            ));
        };
        retained.push(tspp_source::Span::new(
            first_extent.file,
            first_extent.start,
            first_nested.start,
        ));
        retained.push(tspp_source::Span::new(
            first_extent.file,
            first_nested.end,
            first_extent.end,
        ));
        if !module.has_unretained_comment(extent, &retained)? {
            let (Some(prefix), Some(suffix)) = (prefix, suffix) else {
                return Err(ProviderError::internal(
                    "shared nominal or-pattern has no authored wrapper",
                ));
            };
            let alternatives = nested_sources.join(" | ");
            let replacement = format!("{prefix}{alternatives}{suffix}");
            let patch = Patch::replace(extent, replacement);
            let suggestion = lint.fix("nest the alternatives", patch)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Replace repeated nominal tuple patterns.
    #[test]
    fn test_replaces_nominal_tuple_patterns() {
        let session = TestSession::dir(
            &PREFER_NESTED_OR_PATTERN,
            r#"
newtype Count = (int32,);

function isSmall(count: Count): boolean {
    return match (count) {
        Count(0) | Count(1) => true
        _ => false
    };
}
"#,
        );

        session.assert_fixes(
            r#"
newtype Count = (int32,);

function isSmall(count: Count): boolean {
    return match (count) {
        Count(0 | 1) => true
        _ => false
    };
}
"#,
        );
    }

    /// Accept alternatives that select distinct nominal patterns.
    #[test]
    fn test_accepts_distinct_nominal_patterns() {
        let session = TestSession::dir(
            &PREFER_NESTED_OR_PATTERN,
            r#"
function isComplete(result: Result<int32, string>): boolean {
    return match (result) {
        Ok { value: 0 } | Err { error: "done" } => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept patterns that vary more than one field.
    #[test]
    fn test_accepts_multiple_fields() {
        let session = TestSession::dir(
            &PREFER_NESTED_OR_PATTERN,
            r#"
struct Point {
    x: int32;
    y: int32;
}

function liesOnAxis(point: Point): boolean {
    return match (point) {
        Point { x: 0, y } | Point { x, y: 0 } => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
