use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const SORT_METHODS: &[&str] = &["sort", "sortUnstable", "toSorted", "toSortedUnstable"];

declare_lint! {
    /// Disallow comparators that reproduce the natural ordering.
    pub UNNECESSARY_SORT_COMPARATOR {
        id: "unnecessary-sort-comparator",
        summary: "Disallow comparators that reproduce the natural ordering",
        explanation: r#"
A comparator that only calls `left.compare(right)` selects the collection element's natural order.
Instead, you SHOULD omit the comparator and use the natural-order overload.
"#,
        example: {
            reported: r#"
function order(values: int32[]): void {
    values.sortUnstable((left, right) => left.compare(right));
}
"#,
            accepted: r#"
function order(values: int32[]): void {
    values.sortUnstable();
}
"#,
        },
        provenance: [Clippy("unnecessary_sort_by")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report collection comparators that select their element's natural order.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical collection sort calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        if call.is_optional() || !call.generic_arguments.is_empty() || !is_sort_member(member) {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: comparator } = module.view().get(*argument) else {
            continue;
        };
        if !is_natural_comparator(module, *comparator)? {
            continue;
        }

        // remove the comparator argument
        let span = module.source_extent(comparator.into_any())?;
        let mut diagnostic = lint.diagnostic("comparator reproduces natural ordering", span);
        if let Some(suggestion) = suggestion(module, lint, *argument)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one canonical member sorts a collection.
fn is_sort_member(member: dir::LanguageMember) -> bool {
    let is_collection = matches!(
        member.owner,
        dir::LanguageItem::Array | dir::LanguageItem::Slice | dir::LanguageItem::SmallArray
    );

    is_collection
        && SORT_METHODS
            .iter()
            .any(|name| member == member.owner.member(name))
}

/// Return whether one lambda calls compare on its two parameters in order.
fn is_natural_comparator(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some(lambda) = module.lambda(expression) else {
        return Ok(false);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(false);
    }
    let [left, right] = lambda.signature.parameters.as_slice() else {
        return Ok(false);
    };
    if [left, right].iter().any(|parameter| {
        !matches!(
            module.view().get(**parameter),
            dir::Parameter::Named {
                default: None,
                is_optional: false,
                ..
            }
        )
    }) {
        return Ok(false);
    }
    let Some(body) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(false);
    };

    // require the direct natural comparison in parameter order
    let Some(compare) = module.member_call(body) else {
        return Ok(false);
    };
    if compare.is_optional()
        || !compare.generic_arguments.is_empty()
        || module.language_member(body)? != Some(dir::LanguageItem::Compare.member("compare"))
    {
        return Ok(false);
    }
    let [argument] = compare.arguments else {
        return Ok(false);
    };
    let dir::Argument::Positional { value: compared } = module.view().get(*argument) else {
        return Ok(false);
    };
    let left = module.declaration_symbol(*left)?;
    let right = module.declaration_symbol(*right)?;
    let is_natural = module.selected_symbol(compare.receiver)? == Some(left)
        && module.selected_symbol(*compared)? == Some(right);

    Ok(is_natural)
}

/// Delete the sole comparator argument from one sort call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    argument: dir::LocalNodeId<dir::Argument>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(argument.into_any())?;
    if module.has_unretained_comment(extent, &[])? {
        return Ok(None);
    }

    let suggestion = lint.fix("use natural ordering", Patch::delete(extent))?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a comparator that forwards natural ordering.
    #[test]
    fn test_removes_natural_comparator() {
        let session = TestSession::dir(
            &UNNECESSARY_SORT_COMPARATOR,
            r#"
function order(values: int32[]): void {
    values.sortUnstable((left, right) => left.compare(right));
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[unnecessary-sort-comparator]: comparator reproduces natural ordering
 ──▶ main.tspp:2:25
  │
1 │ function order(values: int32[]): void {
2 │     values.sortUnstable((left, right) => left.compare(right));
  │                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use natural ordering
--- a/main.tspp
+++ b/main.tspp

    1│ function order(values: int32[]): void {
-   2│     values.sortUnstable((left, right) => left.compare(right));
+   2│     values.sortUnstable();
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function order(values: int32[]): void {
    values.sortUnstable();
}
"#,
        );
    }

    /// Remove natural comparators from the remaining collection sorts.
    #[test]
    fn test_removes_other_natural_comparators() {
        let session = TestSession::dir(
            &UNNECESSARY_SORT_COMPARATOR,
            r#"
function order(values: ^int32[], slice: &[int32]): ^int32[] {
    values.sortUnstable((left, right) => left.compare(right));
    slice.sort((left, right) => left.compare(right));
    values.toSortedUnstable((left, right) => left.compare(right));
    return values.toSorted((left, right) => left.compare(right));
}
"#,
        );

        session.assert_fixes(
            r#"
function order(values: ^int32[], slice: &[int32]): ^int32[] {
    values.sortUnstable();
    slice.sort();
    values.toSortedUnstable();
    return values.toSorted();
}
"#,
        );
    }

    /// Accept descending and transformed comparators.
    #[test]
    fn test_accepts_distinct_comparators() {
        let session = TestSession::dir(
            &UNNECESSARY_SORT_COMPARATOR,
            r#"
function order(values: int32[]): void {
    values.sortUnstable((left, right) => right.compare(left));
    values.sortUnstable((left, right) => left.absDiff(0).compare(right.absDiff(0)));
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
