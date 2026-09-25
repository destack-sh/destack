use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow spreads that have no observable effect.
    pub NO_USELESS_SPREAD {
        id: "no-useless-spread",
        summary: "Disallow spreads that have no observable effect",
        explanation: r#"
Spreading an inline literal directly into another literal or wrapping an iterable for an API that already accepts it creates an unnecessary intermediate value.
Instead, you SHOULD place the literal entries directly in the list or pass the iterable directly.
"#,
        example: {
            reported: r#"
function append(output: int32[]): void {
    output.push(1, ...[2, 3], 4);
}
"#,
            accepted: r#"
function append(output: int32[]): void {
    output.push(1, 2, 3, 4);
}
"#,
        },
        provenance: [Unicorn("no-useless-spread")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report inline literal spreads and redundant iterable wrappers.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    report_array_spreads(module, lint, &mut output)?;
    report_object_spreads(module, lint, &mut output)?;
    report_iterable_wrappers(module, lint, &mut output)?;

    Ok(output)
}

/// Report inline arrays spread into array literals and argument lists.
fn report_array_spreads(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // inspect spread arguments whose inline arrays can join the surrounding list
    for (argument, node) in view.iter_nodes::<dir::Argument>() {
        let dir::Argument::Spread { value } = node else {
            continue;
        };
        let dir::Expression::ArrayExpression { elements } = view.get(*value) else {
            continue;
        };

        // preserve array holes materialized by spreading
        if elements
            .iter()
            .any(|element| matches!(view.get(*element), dir::Argument::Elision))
        {
            continue;
        }

        // require a surrounding array or argument list
        let Some(parent) = view.get_parent_for(argument) else {
            continue;
        };
        let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
            continue;
        };
        let (description, can_remove_empty) = match view.get(parent) {
            dir::Expression::ArrayExpression { elements: outer } if outer.contains(&argument) => (
                "inline array is spread into an array literal",
                outer.len() == 1,
            ),
            dir::Expression::Call {
                arguments: outer, ..
            }
            | dir::Expression::New {
                arguments: outer, ..
            } if outer.contains(&argument) => (
                "inline array is spread into an argument list",
                outer.len() == 1,
            ),
            _ => continue,
        };

        // replace the spread argument with the inline array elements
        let extent = module.source_extent(argument.into_any())?;
        let mut diagnostic = lint.diagnostic(description, extent);
        if let Some(fix) = inline_literal_fix(
            module,
            lint,
            extent,
            *value,
            '[',
            ']',
            elements.is_empty(),
            can_remove_empty,
        )? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(())
}

/// Report inline objects spread into object literals.
fn report_object_spreads(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // inspect spread objects whose fields can join the surrounding object literal
    for (property, node) in view.iter_nodes::<dir::Property>() {
        let dir::Property::Spread { value } = node else {
            continue;
        };
        let dir::Expression::ObjectExpression { properties } = view.get(*value) else {
            continue;
        };
        if !properties
            .iter()
            .all(|property| matches!(view.get(*property), dir::Property::Field { .. }))
        {
            continue;
        }

        // require a surrounding object literal
        let Some(parent) = view.get_parent_for(property) else {
            continue;
        };
        let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
            continue;
        };
        let dir::Expression::ObjectExpression { properties: outer } = view.get(parent) else {
            continue;
        };
        if !outer.contains(&property) {
            continue;
        }

        // avoid creating duplicate authored keys
        let has_collision = properties.iter().any(|nested| {
            let nested = view.get(*nested).name();
            outer.iter().any(|outer| {
                *outer != property && nested.is_some() && view.get(*outer).name() == nested
            })
        });

        // replace the spread property with the inline object fields
        let extent = module.source_extent(property.into_any())?;
        let mut diagnostic =
            lint.diagnostic("inline object is spread into an object literal", extent);
        if !has_collision
            && let Some(fix) = inline_literal_fix(
                module,
                lint,
                extent,
                *value,
                '{',
                '}',
                properties.is_empty(),
                outer.len() == 1,
            )?
        {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(())
}

/// Report array wrappers passed to canonical iterable consumers.
fn report_iterable_wrappers(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // inspect array wrappers passed to canonical iterable consumers
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Call { .. }) {
            continue;
        }
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() || call.arguments.len() != 1 {
            continue;
        }

        // require a known collection conversion
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        let is_collection_from = [
            dir::LanguageItem::Array,
            dir::LanguageItem::Map,
            dir::LanguageItem::Set,
            dir::LanguageItem::Slice,
        ]
        .into_iter()
        .any(|item| member == item.member("from"));
        if !is_collection_from {
            continue;
        }

        // require the selected parameter to consume an iterable
        let Some(parameters) = module.call_parameters(expression)? else {
            continue;
        };
        let Some(parameter) = parameters.first() else {
            continue;
        };
        if module.dir.representation_item(parameter.ty)? != Some(dir::LanguageItem::Iterable) {
            continue;
        }

        // select one array containing one spread value
        let dir::Argument::Positional { value: wrapper } = view.get(call.arguments[0]) else {
            continue;
        };
        let Some(value) = single_spread_array(module, *wrapper) else {
            continue;
        };

        // replace the temporary array with its iterable value
        let extent = module.source_extent(wrapper.into_any())?;
        let mut diagnostic =
            lint.diagnostic("iterable is copied before an iterable argument", extent);
        if let Some(fix) = wrapper_fix(module, lint, extent, value)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(())
}

/// Return the value wrapped by an array containing one spread element.
fn single_spread_array(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let dir::Expression::ArrayExpression { elements } = module.view().get(expression) else {
        return None;
    };
    let [element] = elements.as_slice() else {
        return None;
    };
    let dir::Argument::Spread { value } = module.view().get(*element) else {
        return None;
    };

    Some(*value)
}

/// Build a replacement that joins one inline literal with its surrounding list.
fn inline_literal_fix(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    literal: dir::LocalNodeId<dir::Expression>,
    opening: char,
    closing: char,
    is_empty: bool,
    can_remove_empty: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // require a directly written literal that can occupy its list position
    if (is_empty && !can_remove_empty) || module.source_parentheses(literal.into_any()).is_some() {
        return Ok(None);
    }

    // require the authored delimiters around the literal contents
    let literal = module.source_extent(literal.into_any())?;
    let source = module.source(literal)?;
    if !source.starts_with(opening) || !source.ends_with(closing) {
        return Err(ProviderError::internal(format!(
            "inline literal {literal:?} does not contain its authored delimiters"
        )));
    }

    // remove the literal delimiters and a trailing separator
    let contents = Span::new(literal.file, literal.start + 1, literal.end - 1);
    let contents = module.source(contents)?.trim();
    let contents = contents.strip_suffix(',').unwrap_or(contents).trim_end();

    let patch = Patch::replace(extent, contents);
    let fix = lint.fix("inline the spread literal", patch)?;

    Ok(Some(fix))
}

/// Build a replacement that removes one temporary array wrapper.
fn wrapper_fix(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the iterable expression with its authored grouping
    let replacement = module.expression_source(value, dir::OperatorPrecedence::Lowest)?;
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("pass the iterable directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Inline array literals in array and call argument lists.
    #[test]
    fn test_replaces_inline_arrays() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function append(output: int32[]): int32[] {
    output.push(1, ...[2, 3], 4);
    return [0, ...[1, 2], 3];
}
"#,
        );

        session.assert_fixes(
            r#"
function append(output: int32[]): int32[] {
    output.push(1, 2, 3, 4);
    return [0, 1, 2, 3];
}
"#,
        );
    }

    /// Inline object fields in an object literal.
    #[test]
    fn test_replaces_inline_object() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function point(x: int32, y: int32): { x: int32; y: int32 } {
    return { ...{ x }, y };
}
"#,
        );

        session.assert_fixes(
            r#"
function point(x: int32, y: int32): { x: int32; y: int32 } {
    return { x, y };
}
"#,
        );
    }

    /// Remove an array wrapper from a canonical iterable consumer.
    #[test]
    fn test_replaces_iterable_wrapper() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
import { Set } from "tspp:collections";

function unique(values: int32[]): Set<int32> {
    return Set.from([...values]);
}
"#,
        );

        session.assert_fixes(
            r#"
import { Set } from "tspp:collections";

function unique(values: int32[]): Set<int32> {
    return Set.from(values);
}
"#,
        );
    }

    /// Keep sparse inline arrays because spreading materializes their holes.
    #[test]
    fn test_accepts_sparse_inline_array() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function values(): (int32 | undefined)[] {
    return [0, ...[1, , 2], 3];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a comment in an empty inline array by omitting the fix.
    #[test]
    fn test_reports_commented_empty_array_without_fix() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function values(): int32[] {
    return [1, ...[/* retain */], 2];
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-spread]: inline array is spread into an array literal
 ──▶ main.tspp:2:16
  │
1 │ function values(): int32[] {
2 │     return [1, ...[/* retain */], 2];
  │                ^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Keep arbitrary iterable wrappers where the callee's behavior is unknown.
    #[test]
    fn test_accepts_user_iterable_consumer() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function consume(values: Iterable<int32>): void {
    values;
}

function run(values: int32[]): void {
    consume([...values]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep an Array.from wrapper when a mapper observes source iteration.
    #[test]
    fn test_accepts_mapped_array_from_wrapper() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
function copy(values: int32[]): int32[] {
    return Array.from([...values], (value) => value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep an object spread whose accessor semantics differ from a direct field.
    #[test]
    fn test_accepts_inline_object_method() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
const value = { ...{ get count(): int32 { return 1; } } };
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report colliding inline fields without producing duplicate authored keys.
    #[test]
    fn test_reports_colliding_object_field_without_fix() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
const value = { ...{ count: 1 }, count: 2 };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-spread]: inline object is spread into an object literal
 ──▶ main.tspp:1:17
  │
1 │ const value = { ...{ count: 1 }, count: 2 };
  │                 ^^^^^^^^^^^^^^^
  │
"#,
        );
    }

    /// Preserve comments around a wrapped iterable by omitting the fix.
    #[test]
    fn test_reports_commented_wrapper_without_fix() {
        let session = TestSession::dir(
            &NO_USELESS_SPREAD,
            r#"
import { Set } from "tspp:collections";

function unique(values: int32[]): Set<int32> {
    return Set.from([/* retain */ ...values]);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-spread]: iterable is copied before an iterable argument
 ──▶ main.tspp:4:21
  │
2 │
3 │ function unique(values: int32[]): Set<int32> {
4 │     return Set.from([/* retain */ ...values]);
  │                     ^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }
}
