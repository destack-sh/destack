use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer filterMap when removing undefined values from a mapped array.
    pub MANUAL_FILTER_MAP {
        id: "manual-filter-map",
        summary: "Prefer filterMap when removing undefined values from a mapped array",
        explanation: r#"
Mapping an array before filtering out undefined results allocates an intermediate array.
Instead, you SHOULD call `filterMap` to keep defined mapped values in one operation.
"#,
        example: {
            reported: r#"
declare const values: (int32 | undefined)[];

const defined = values.map((value) => value).filter((value) => value !== undefined);
"#,
            accepted: r#"
declare const values: (int32 | undefined)[];

const defined = values.filterMap((value) => value);
"#,
        },
        provenance: [Clippy("manual_filter_map")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report mapped arrays filtered by an exact defined-value predicate.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array.filter calls with one predicate
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(filter) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("filter")) {
            continue;
        }
        let [predicate] = filter.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: predicate } = view.get(*predicate) else {
            continue;
        };
        if !module.is_defined_predicate(*predicate)? {
            continue;
        }

        // require the filtered receiver to be one canonical Array.map call
        let Some(map) = module.member_call(filter.receiver) else {
            continue;
        };
        if map.arguments.len() != 1
            || module.language_member(filter.receiver)?
                != Some(dir::LanguageItem::Array.member("map"))
        {
            continue;
        }

        // fuse mapping and defined-value filtering
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("mapped array is filtered only for defined values", span);
        if let Some(suggestion) = suggestion(module, lint, expression, filter.receiver, map.callee)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one filterMap call from a mapped defined-value filter.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    map: dir::LocalNodeId<dir::Expression>,
    map_callee: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let map_extent = module.source_extent(map.into_any())?;
    if !extent.contains_span(map_extent) {
        return Err(ProviderError::internal(
            "mapped filter extent does not contain its map call",
        ));
    }
    if module.has_unretained_comment(extent, &[map_extent])? {
        return Ok(None);
    }

    // replace the map callee and remove the defined-value filter suffix
    let suffix = Span::new(extent.file, map_extent.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.replace(module.main_span(map_callee.into_any())?, "filterMap");
    file.delete(suffix);
    file.sort();
    let suggestion = lint.suggestion("map and keep defined values together", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Fuse a map followed by an exact defined-value filter.
    #[test]
    fn test_reports_mapped_defined_filter() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.map((value) => value).filter((value) => value !== undefined);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-filter-map]: mapped array is filtered only for defined values
 ──▶ main.tspp:2:12
  │
1 │ function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
2 │     return values.map((value) => value).filter((value) => value !== undefined);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: map and keep defined values together (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
-   2│     return values.map((value) => value).filter((value) => value !== undefined);
+   2│     return values.filterMap((value) => value);
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.filterMap((value) => value);
}
"#,
        );
    }

    /// Recognize undefined on the left of a strict comparison.
    #[test]
    fn test_reports_reversed_defined_filter() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.map((value) => value).filter((value) => undefined !== value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.filterMap((value) => value);
}
"#,
        );
    }

    /// Preserve an optional Array receiver while fusing map and filter.
    #[test]
    fn test_reports_optional_map() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
function defined(
    values: (int32 | undefined)[] | undefined,
): (int32 | undefined)[] | undefined {
    return values?.map((value) => value).filter((value) => value !== undefined);
}
"#,
        );

        session.assert_suggestions(
            r#"
function defined(
    values: (int32 | undefined)[] | undefined,
): (int32 | undefined)[] | undefined {
    return values?.filterMap((value) => value);
}
"#,
        );
    }

    /// Accept loose undefined comparison because it also rejects null.
    #[test]
    fn test_accepts_loose_undefined_filter() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
function defined(values: (int32 | null | undefined)[]): void {
    const filtered = values.map((value) => value).filter((value) => value != undefined);
    filtered;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that filters values for another reason.
    #[test]
    fn test_accepts_other_filter() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
function positive(values: int32[]): int32[] {
    return values.map((value) => value + 1).filter((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined map and filter methods.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &MANUAL_FILTER_MAP,
            r#"
class Values {
    map(mapper: (value: int32) => int32 | undefined): this {
        return this;
    }

    filter(predicate: (value: int32 | undefined) => boolean): this {
        return this;
    }
}

function parse(values: Values): Values {
    return values.map((value) => value).filter((value) => value !== undefined);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
