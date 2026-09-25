use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow imports, exports, and destructuring fields renamed to the same name.
    pub NO_USELESS_RENAME {
        id: "no-useless-rename",
        summary: "Disallow imports, exports, and destructuring fields renamed to the same name",
        explanation: r#"
Renaming an import, export, or destructured field to its existing name adds syntax without changing the introduced binding.
Instead, you SHOULD use the corresponding shorthand form.
"#,
        example: {
            reported: r#"
function name(user: { name: string }): string {
    const { name: name } = user;
    return name;
}
"#,
            accepted: r#"
function name(user: { name: string }): string {
    const { name } = user;
    return name;
}
"#,
        },
        provenance: [Eslint("no-useless-rename")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report explicit aliases that repeat their source name.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect named import and export items
    for (item, node) in view.iter_nodes::<dir::DependencyItem>() {
        let dir::DependencyItem::Binding {
            name: Some(name @ (dir::Name::Identifier(source) | dir::Name::String(source))),
            alias: Some(alias),
            ..
        } = node
        else {
            continue;
        };
        if source != alias {
            continue;
        }

        let span = module.source_extent(item.into_any())?;
        let mut diagnostic = lint.diagnostic("dependency alias repeats its source name", span);
        if let Some(suggestion) =
            suggest_dependency_shorthand(module, lint, item.into_any(), *name)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    // inspect binding destructuring fields
    for (field, node) in view.iter_nodes::<dir::PatternField>() {
        let dir::PatternField::Named {
            name: dir::Name::Identifier(name) | dir::Name::String(name),
            pattern: Some(pattern),
            is_shorthand: false,
        } = node
        else {
            continue;
        };
        let Some(binding) = find_binding(&view, *pattern, *name) else {
            continue;
        };

        report_destructuring_rename(
            module,
            lint,
            field.into_any(),
            binding.into_any(),
            &mut output,
        )?;
    }

    // inspect destructuring assignment fields
    for (field, node) in view.iter_nodes::<dir::AssignPatternField>() {
        let dir::AssignPatternField::Named {
            name: dir::Name::Identifier(name) | dir::Name::String(name),
            pattern,
            is_shorthand: false,
        } = node
        else {
            continue;
        };
        let Some(place) = find_place(&view, *pattern, *name) else {
            continue;
        };

        report_destructuring_rename(
            module,
            lint,
            field.into_any(),
            place.into_any(),
            &mut output,
        )?;
    }

    Ok(output)
}

/// Return the matching binding nested beneath one direct field rename.
fn find_binding(
    view: &dir::View<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    name: dir::StringId,
) -> Option<dir::LocalNodeId<dir::Pattern>> {
    match view.get(pattern) {
        dir::Pattern::Binding {
            name: binding,
            pattern: None,
        } if *binding == name => Some(pattern),
        dir::Pattern::Default { pattern, .. } => find_binding(view, *pattern, name),
        _ => None,
    }
}

/// Return the matching place nested beneath one direct assignment rename.
fn find_place(
    view: &dir::View<'_>,
    pattern: dir::LocalNodeId<dir::AssignPattern>,
    name: dir::StringId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match view.get(pattern) {
        dir::AssignPattern::Place { expression } => match view.get(*expression) {
            dir::Expression::Identifier { name: place } if *place == name => Some(*expression),
            _ => None,
        },
        dir::AssignPattern::Default { pattern, .. } => find_place(view, *pattern, name),
        _ => None,
    }
}

/// Report one redundant destructuring rename and retain its binding or place.
fn report_destructuring_rename(
    module: &DirModule<'_>,
    lint: &Lint,
    field: dir::LocalNodeIdAny,
    retained: dir::LocalNodeIdAny,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let span = module.source_extent(field)?;
    let mut diagnostic = lint.diagnostic("destructured field repeats its source name", span);
    if let Some(suggestion) = suggest_destructuring_shorthand(module, lint, field, retained)? {
        diagnostic = diagnostic.suggestion(suggestion);
    }
    output.report(diagnostic);

    Ok(())
}

/// Build the shorthand form of one dependency item.
fn suggest_dependency_shorthand(
    module: &DirModule<'_>,
    lint: &Lint,
    item: dir::LocalNodeIdAny,
    name: dir::Name,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // select an identifier spelling that remains valid without an alias
    let source = module.source_region(item, NodeSpanRegion::Type)?;
    let alias = module.main_span(item)?;
    let retained = match name {
        dir::Name::Identifier(_) => source,
        dir::Name::String(_) => {
            let alias_source = module.source(alias)?;
            if alias_source.starts_with('"') || alias_source.starts_with('\'') {
                return Ok(None);
            }

            alias
        }
        dir::Name::Index(_) => return Ok(None),
    };
    let replacement = module.source(retained)?;

    // preserve comments outside the retained shorthand name
    let redundant = Span::new(source.file, source.start, alias.end);
    if module.has_unretained_comment(redundant, &[retained])? {
        return Ok(None);
    }

    // replace the complete rename with its valid shorthand name
    let patch = Patch::replace(redundant, replacement);
    let suggestion = lint.suggestion("remove the redundant alias", patch)?;

    Ok(Some(suggestion))
}

/// Build the shorthand form of one destructuring field.
fn suggest_destructuring_shorthand(
    module: &DirModule<'_>,
    lint: &Lint,
    field: dir::LocalNodeIdAny,
    retained: dir::LocalNodeIdAny,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve comments outside the retained identifier
    let key = module.main_span(field)?;
    let retained = module.main_span(retained)?;
    let redundant = Span::new(key.file, key.start, retained.end);
    if module.has_unretained_comment(redundant, &[retained])? {
        return Ok(None);
    }

    // replace the complete rename with its shorthand identifier
    let patch = Patch::replace(redundant, module.source(retained)?);
    let suggestion = lint.suggestion("use destructuring shorthand", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a rename that introduces a distinct local name.
    #[test]
    fn test_accepts_distinct_name() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function name(user: { name: string }): string {
    const { name: displayName } = user;
    return displayName;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve the default when recognizing a redundant rename.
    #[test]
    fn test_replaces_defaulted_rename() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function name(user: { name?: string }): string {
    const { name: name = "anonymous" } = user;
    return name;
}
"#,
        );

        session.assert_suggestions(
            r#"
function name(user: { name?: string }): string {
    const { name = "anonymous" } = user;
    return name;
}
"#,
        );
    }

    /// Replace a quoted destructuring key with its shorthand binding.
    #[test]
    fn test_replaces_quoted_field_rename() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function name(user: { name: string }): string {
    const { "name": name } = user;
    return name;
}
"#,
        );

        session.assert_suggestions(
            r#"
function name(user: { name: string }): string {
    const { name } = user;
    return name;
}
"#,
        );
    }

    /// Replace a redundant local export alias.
    #[test]
    fn test_replaces_export_alias() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
const version = 1;
export { version as version };
"#,
        );

        session.assert_suggestions(
            r#"
const version = 1;
export { version };
"#,
        );
    }

    /// Replace a quoted export alias with its source identifier.
    #[test]
    fn test_replaces_quoted_export_alias() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
const version = 1;
export { version as "version" };
"#,
        );

        session.assert_suggestions(
            r#"
const version = 1;
export { version };
"#,
        );
    }

    /// Replace a redundant import alias.
    #[test]
    fn test_replaces_import_alias() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
import { Array as Array } from "tspp:collections";
"#,
        );

        session.assert_suggestions(
            r#"
import { Array } from "tspp:collections";
"#,
        );
    }

    /// Replace a quoted import name with its equivalent local identifier.
    #[test]
    fn test_replaces_quoted_import_name() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
import { "Array" as Array } from "tspp:collections";
"#,
        );

        session.assert_suggestions(
            r#"
import { Array } from "tspp:collections";
"#,
        );
    }

    /// Replace a redundant destructuring assignment rename.
    #[test]
    fn test_replaces_assignment_rename() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function read(point: { x: int32 }): int32 {
    let x: int32 = 0;
    ({ x: x } = point);
    return x;
}
"#,
        );

        session.assert_suggestions(
            r#"
function read(point: { x: int32 }): int32 {
    let x: int32 = 0;
    ({ x } = point);
    return x;
}
"#,
        );
    }

    /// Replace a quoted assignment key with its shorthand place.
    #[test]
    fn test_replaces_quoted_assignment_rename() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function read(point: { x: int32 }): int32 {
    let x: int32 = 0;
    ({ "x": x } = point);
    return x;
}
"#,
        );

        session.assert_suggestions(
            r#"
function read(point: { x: int32 }): int32 {
    let x: int32 = 0;
    ({ x } = point);
    return x;
}
"#,
        );
    }

    /// Preserve a comment within a redundant field rename by omitting the suggestion.
    #[test]
    fn test_reports_commented_field_without_suggestion() {
        let session = TestSession::dir(
            &NO_USELESS_RENAME,
            r#"
function name(user: { name: string }): string {
    const { name: /* retain */ name } = user;
    return name;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-rename]: destructured field repeats its source name
 ──▶ main.tspp:2:13
  │
1 │ function name(user: { name: string }): string {
2 │     const { name: /* retain */ name } = user;
  │             ^^^^^^^^^^^^^^^^^^^^^^^
3 │     return name;
4 │ }
  │
"#,
        );
    }
}
