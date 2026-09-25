use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer retain over replacing an owned array with its filtered result.
    pub MANUAL_RETAIN {
        id: "manual-retain",
        summary: "Prefer retain over replacing an owned array with its filtered result",
        explanation: r#"
Assigning an owned array's filtered result back to the same place allocates replacement storage.
Instead, you SHOULD call `retain` to remove rejected elements from the owned array in place.
"#,
        example: {
            reported: r#"
function keepPositive(input: ^int32[]): ^int32[] {
    let values = input;
    values = values.filter((value) => value > 0);
    return values;
}
"#,
            accepted: r#"
function keepPositive(input: ^int32[]): ^int32[] {
    let values = input;
    values.retain((value) => value > 0);
    return values;
}
"#,
        },
        provenance: [Clippy("manual_retain")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report owned arrays replaced by their own filtered result.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect statement-position plain assignments
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }
        if !module.is_discarded_expression(expression) {
            continue;
        }

        // require a canonical filter over the exact written storage path
        let Some(filter) = module.member_call(assignment.value) else {
            continue;
        };
        if filter.is_optional()
            || module.language_member(assignment.value)?
                != Some(dir::LanguageItem::Array.member("filter"))
        {
            continue;
        }
        if module.access_resolution(assignment.target) != module.access_resolution(filter.receiver)
            || module.access_resolution(assignment.target).is_none()
            || !module.is_duplicable_expression(assignment.target)?
        {
            continue;
        }
        let target_type = module.adjusted_type(assignment.target.into_any())?;
        let receiver_type = module.adjusted_type(filter.receiver.into_any())?;
        if target_type != receiver_type
            || !matches!(target_type, dir::Type::Form(form) if form.form == dir::Form::Owned)
        {
            continue;
        }
        let [predicate] = filter.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: predicate } = view.get(*predicate) else {
            continue;
        };

        // replace rebuilding assignment with in-place retention
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("owned array is replaced by its own filtered values", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, assignment.target, *predicate)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one retain call from a filtering self-assignment.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    predicate: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target_extent = module.source_extent(target.into_any())?;
    let predicate_extent = module.source_extent(predicate.into_any())?;
    if module.has_unretained_comment(extent, &[target_extent, predicate_extent])? {
        return Ok(None);
    }

    // preserve the authored target and predicate
    let target = module.expression_source(target, dir::OperatorPrecedence::Postfix)?;
    let predicate = module.source(predicate_extent)?;
    let replacement = format!("{target}.retain({predicate})");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("retain matching values in place", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an owned Array filtering self-assignment with retain.
    #[test]
    fn test_replaces_filtering_self_assignment() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
function keepPositive(input: ^int32[]): ^int32[] {
    let values = input;
    values = values.filter((value) => value > 0);
    return values;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-retain]: owned array is replaced by its own filtered values
 ──▶ main.tspp:3:5
  │
1 │ function keepPositive(input: ^int32[]): ^int32[] {
2 │     let values = input;
3 │     values = values.filter((value) => value > 0);
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     return values;
5 │ }
  │

 = suggestion: retain matching values in place (requires review)
--- a/main.tspp
+++ b/main.tspp

    2│     let values = input;
-   3│     values = values.filter((value) => value > 0);
+   3│     values.retain((value) => value > 0);
    4│     return values;
"#,
        );
        session.assert_suggestions(
            r#"
function keepPositive(input: ^int32[]): ^int32[] {
    let values = input;
    values.retain((value) => value > 0);
    return values;
}
"#,
        );
    }

    /// Accept filtering a different source collection.
    #[test]
    fn test_accepts_different_source() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
function replace(target: int32[], source: int32[]): int32[] {
    let result = target;
    result = source.filter((value) => value > 0);
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a managed Array because retain would mutate its aliases.
    #[test]
    fn test_accepts_managed_array() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
function keepPositive(input: int32[]): int32[] {
    let values = input;
    values = values.filter((value) => value > 0);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a filtered value returned without assignment.
    #[test]
    fn test_accepts_filter_result() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
function positives(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined filter method.
    #[test]
    fn test_accepts_user_filter() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
class Values {
    filter(predicate: (value: int32) => boolean): this {
        return this;
    }
}

function keep(input: Values): Values {
    let values = input;
    values = values.filter((value) => value > 0);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept the default implementation of a clone interface.
    #[test]
    fn test_accepts_default_clone_implementation() {
        let session = TestSession::dir(
            &MANUAL_RETAIN,
            r#"
newtype interface Duplicate {
    clone(&readonly this): ^this;

    cloneFrom(&this, source: &readonly this): void {
        *this = source.clone();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
