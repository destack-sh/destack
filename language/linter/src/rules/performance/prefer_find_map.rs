use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer findMap over separate mapping and search operations.
    pub PREFER_FIND_MAP {
        id: "prefer-find-map",
        summary: "Prefer findMap over separate mapping and search operations",
        explanation: r#"
Mapping before a search creates a separate Iterator adapter and traversal state.
Instead, you SHOULD call `findMap` to transform values until the first defined result is produced.
"#,
        example: {
            reported: r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .map((value) => value)
        .find((value) => value !== undefined);
}
"#,
            accepted: r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .findMap((value) => value);
}
"#,
        },
        provenance: [
            Clippy("filter_map_next"),
            Clippy("manual_find_map"),
        ],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Iterator operations that can become one findMap call.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect Iterator operations that collapse into findMap
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some((adapter, callee)) = find_map_adapter(module, expression)? else {
            continue;
        };

        // fuse the adapter and terminal operation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic(
            "Iterator maps values before selecting the first defined result",
            span,
        );
        if let Some(suggestion) = suggestion(module, lint, expression, adapter, callee)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the adapter and callee that can become one findMap call.
fn find_map_adapter(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<
    Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let Some(terminal) = module.member_call(expression) else {
        return Ok(None);
    };
    let member = module.language_member(expression)?;

    // recognize map followed by an exact defined-value search
    let adapter_name = if member == Some(dir::LanguageItem::Iterator.member("find")) {
        let [argument] = terminal.arguments else {
            return Ok(None);
        };
        let dir::Argument::Positional { value: predicate } = module.view().get(*argument) else {
            return Ok(None);
        };
        if !module.is_defined_predicate(*predicate)? {
            return Ok(None);
        }

        "map"
    }
    // recognize filterMap followed by first
    else if member == Some(dir::LanguageItem::Iterator.member("first")) {
        if !terminal.arguments.is_empty() {
            return Ok(None);
        }

        "filterMap"
    } else {
        return Ok(None);
    };

    // require the canonical single-mapper adapter
    let Some(adapter) = module.member_call(terminal.receiver) else {
        return Ok(None);
    };
    if adapter.arguments.len() != 1
        || module.language_member(terminal.receiver)?
            != Some(dir::LanguageItem::Iterator.member(adapter_name))
    {
        return Ok(None);
    }

    Ok(Some((terminal.receiver, adapter.callee)))
}

/// Build one findMap call from an adapter and terminal operation.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    adapter: dir::LocalNodeId<dir::Expression>,
    adapter_callee: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let adapter_extent = module.source_extent(adapter.into_any())?;
    if !extent.contains_span(adapter_extent) {
        return Err(ProviderError::internal(
            "fused Iterator extent does not contain its adapter",
        ));
    }
    if module.has_unretained_comment(extent, &[adapter_extent])? {
        return Ok(None);
    }

    // replace the adapter and remove the terminal suffix
    let suffix = Span::new(extent.file, adapter_extent.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.replace(module.main_span(adapter_callee.into_any())?, "findMap");
    file.delete(suffix);
    file.sort();
    let suggestion = lint.fix("transform until the first defined value", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Recognize undefined on the left of the strict comparison.
    #[test]
    fn test_replaces_reversed_defined_search() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .map((value) => value)
        .find((value) => undefined !== value);
}
"#,
        );

        session.assert_fixes(
            r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .findMap((value) => value);
}
"#,
        );
    }

    /// Preserve mapper index behavior while removing the search predicate.
    #[test]
    fn test_replaces_indexed_mapping() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstDefined(values: int32[]): int32 | undefined {
    return values
        .iterator()
        .map((value, index) => index > 0 ? value : undefined)
        .find((value) => value !== undefined);
}
"#,
        );

        session.assert_fixes(
            r#"
function firstDefined(values: int32[]): int32 | undefined {
    return values
        .iterator()
        .findMap((value, index) => index > 0 ? value : undefined);
}
"#,
        );
    }

    /// Fuse filterMap followed by first into findMap.
    #[test]
    fn test_replaces_filtered_first() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.iterator().filterMap((value) => value > 0 ? value : undefined).first();
}
"#,
        );

        session.assert_fixes(
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.iterator().findMap((value) => value > 0 ? value : undefined);
}
"#,
        );
    }

    /// Preserve an optional Iterator receiver while fusing map and find.
    #[test]
    fn test_replaces_optional_mapping() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
import { Iterator } from "tspp:iter";

function firstDefined(values: Iterator<int32 | undefined> | undefined): int32 | undefined {
    return values?.map((value) => value).find((value) => value !== undefined);
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "tspp:iter";

function firstDefined(values: Iterator<int32 | undefined> | undefined): int32 | undefined {
    return values?.findMap((value) => value);
}
"#,
        );
    }

    /// Recognize a constant alias of undefined.
    #[test]
    fn test_replaces_undefined_constant() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
const missing = undefined;

function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values.iterator().map((value) => value).find((value) => value !== missing);
}
"#,
        );

        session.assert_fixes(
            r#"
const missing = undefined;

function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values.iterator().findMap((value) => value);
}
"#,
        );
    }

    /// Accept a search predicate with preceding effects.
    #[test]
    fn test_accepts_effectful_search() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .map((value) => value)
        .find((value) => {
            value;
            return value !== undefined;
        });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that rejects more than undefined.
    #[test]
    fn test_accepts_additional_predicate() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstPositive(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .map((value) => value)
        .find((value) => value !== undefined && value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments between the adapter and terminal operation.
    #[test]
    fn test_reports_commented_terminal_without_fix() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
    return values
        .iterator()
        .map((value) => value) /* retain */
        .find((value) => value !== undefined);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-find-map]: Iterator maps values before selecting the first defined result
 ──▶ main.tspp:2:12
  │
1 │ function firstDefined(values: (int32 | undefined)[]): int32 | undefined {
2 │     return values
  │            ^^^^^^
3 │         .iterator()
  │         ^^^^^^^^^^^
4 │         .map((value) => value) /* retain */
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │         .find((value) => value !== undefined);
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }

    /// Accept user-defined methods with the same names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &PREFER_FIND_MAP,
            r#"
class Values {
    map(mapper: (value: int32) => int32 | undefined): this {
        return this;
    }

    find(predicate: (value: int32 | undefined) => boolean): int32 | undefined {
        return undefined;
    }
}

function firstDefined(values: Values): int32 | undefined {
    return values.map((value) => value).find((value) => value !== undefined);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
