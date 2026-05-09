use crate::LintMeta;
use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow renaming import, export, and destructured assignments to the same name.
    ///
    /// Using `{x: x}` in destructuring is the same as `{x}` and is unnecessarily verbose.
    #[lint(
        id = "no-useless-rename",
        code = "LU040",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessRename,
    "Disallow useless renaming"
}

impl LintRule for NoUselessRename {
    fn meta(&self) -> &'static LintMeta {
        NoUselessRename::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // check destructuring pattern fields
        if !ctx
            .options
            .correctness
            .no_useless_rename_ignore_destructuring
        {
            for node_id in ctx.tree.iter_nodes::<ast::PatternField>() {
                let field = ctx.tree.get(node_id);

                // keep only expanded named fields with one identifier alias
                let Some((mutability, name_id, default_expression_id)) =
                    useless_destructuring_alias_parts(ctx, field)
                else {
                    continue;
                };

                // enforce one identical alias pair
                // resolve effective severity
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // build one diagnostic message
                let name_text: String = ctx.strings.get(name_id).to_string();
                let field_span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintReport::new(
                    NO_USELESS_RENAME.id,
                    NO_USELESS_RENAME.code,
                    NO_USELESS_RENAME.category,
                    severity,
                    format!("useless rename: `{name_text}: {name_text}` can be `{name_text}`"),
                    field_span,
                )
                .label("this rename is unnecessary");

                // attach one fix for safe local rewrites
                if ctx.compute_fixes
                    && let Some(fix) = useless_destructuring_rename_fix(
                        ctx,
                        field_span,
                        mutability,
                        name_text,
                        default_expression_id,
                    )
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }

        // check import and export dependency items
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let (items, rename_kind) = match expression {
                ast::Expression::Import { items, .. } => {
                    let Some(items) = items.as_deref() else {
                        continue;
                    };

                    (items, RenameKind::Import)
                }
                ast::Expression::Export { items, .. } => (items.as_slice(), RenameKind::Export),
                _ => continue,
            };

            // honor per-kind ignore options
            if (rename_kind == RenameKind::Import
                && ctx.options.correctness.no_useless_rename_ignore_import)
                || (rename_kind == RenameKind::Export
                    && ctx.options.correctness.no_useless_rename_ignore_export)
            {
                continue;
            }

            // inspect each renamed item in the clause
            for item_id in items {
                let item = ctx.tree.get(*item_id);
                let ast::DependencyItem::Item {
                    alias: Some(alias_id),
                    name: Some(name),
                    ..
                } = item
                else {
                    continue;
                };
                let ast::Name::Identifier(name_id) = name else {
                    continue;
                };
                if *name_id != *alias_id {
                    continue;
                }

                // resolve effective severity
                let severity = ctx.get_effective_severity(meta, *item_id);
                if !severity.is_enabled() {
                    continue;
                }

                // build one diagnostic for this dependency item
                let name_text: String = ctx.strings.get(*name_id).to_string();
                let item_span = ctx.tree.get_span(*item_id);
                let mut diagnostic = LintReport::new(
                    NO_USELESS_RENAME.id,
                    NO_USELESS_RENAME.code,
                    NO_USELESS_RENAME.category,
                    severity,
                    format!(
                        "useless rename: {} `{name_text}` renamed to itself",
                        rename_kind.label()
                    ),
                    item_span,
                )
                .label("this rename is unnecessary");

                // attach one safe fix when text shape is trivial and comment free
                if ctx.compute_fixes
                    && let Some(fix) = useless_dependency_item_rename_fix(ctx, item_span)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return the shorthand-collapse parts for one destructuring rename.
fn useless_destructuring_alias_parts(
    ctx: &LintAstContext<'_>,
    field: &ast::PatternField,
) -> Option<(
    Option<ast::Mutability>,
    ast::StringId,
    Option<ast::LocalNodeId<ast::Expression>>,
)> {
    let ast::PatternField::Named {
        mutability,
        name,
        is_shorthand,
        pattern: Some(pattern_id),
    } = field
    else {
        return None;
    };

    if *is_shorthand {
        return None;
    }

    let ast::Name::Identifier(name_id) = name else {
        return None;
    };

    let (binding_name, default_expression_id) =
        destructuring_alias_pattern_parts(ctx, *pattern_id)?;

    if *name_id != binding_name {
        return None;
    }

    Some((*mutability, *name_id, default_expression_id))
}

/// Return the nested binding name and default for one alias pattern.
fn destructuring_alias_pattern_parts(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<(ast::StringId, Option<ast::LocalNodeId<ast::Expression>>)> {
    match ctx.tree.get(pattern_id) {
        ast::Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some((*name, None)),
        ast::Pattern::Assign { pattern, value } => {
            let ast::Pattern::Binding {
                name,
                pattern: None,
                ..
            } = ctx.tree.get(*pattern)
            else {
                return None;
            };

            Some((*name, Some(*value)))
        }
        _ => None,
    }
}

/// One dependency rename source kind for messages.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenameKind {
    /// Import item rename.
    Import,
    /// Export item rename.
    Export,
}

impl RenameKind {
    /// Return the lower case kind label.
    fn label(self) -> &'static str {
        match self {
            RenameKind::Import => "import",
            RenameKind::Export => "export",
        }
    }
}

/// Build one safe fix for a useless destructuring alias.
fn useless_destructuring_rename_fix(
    ctx: &LintAstContext<'_>,
    field_span: Span,
    mutability: Option<ast::Mutability>,
    name_text: String,
    default_expression_id: Option<ast::LocalNodeId<ast::Expression>>,
) -> Option<LintFix> {
    // avoid touching commented nodes
    if span_has_comment(ctx.tree, field_span) {
        return None;
    }

    // keep local mutability keyword in shorthand replacements
    let mutability_prefix = match mutability {
        Some(ast::Mutability::Immutable) => "const ",
        Some(ast::Mutability::Mutable) => "var ",
        None => "",
    };

    // keep default expressions in shorthand shape
    let replacement = if let Some(default_expression_id) = default_expression_id {
        let default_span = ctx.tree.get_span(default_expression_id);
        let default_text = ctx.get_span_text(default_span);
        format!("{mutability_prefix}{name_text} = {default_text}")
    } else {
        format!("{mutability_prefix}{name_text}")
    };

    // build fix edits
    let edits = ctx
        .edit_builder()
        .replace(field_span, replacement)
        .into_edits();
    Some(LintFix::safe("Remove useless rename").with_edits(edits))
}

/// Build one safe fix for a useless import or export rename item.
fn useless_dependency_item_rename_fix(
    ctx: &LintAstContext<'_>,
    item_span: Span,
) -> Option<LintFix> {
    // avoid touching commented items
    if span_has_comment(ctx.tree, item_span) {
        return None;
    }

    // resolve the current item source text
    let item_text = ctx.get_span_text(item_span);
    let separator_index = item_text.rfind(" as ")?;
    let replacement = item_text[..separator_index].trim_end().to_string();
    if replacement.is_empty() {
        return None;
    }

    // build fix edits
    let edits = ctx
        .edit_builder()
        .replace(item_span, replacement)
        .into_edits();
    Some(LintFix::safe("Remove useless rename").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_useless_rename_destructure() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_detects_useless_rename_destructure.ds",
            r#"
const { x: x } = obj
"#,
        );
        test.result(result).assert_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_actual_rename() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_allows_actual_rename.ds",
            r#"
const { x: y } = obj
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_simple_destructure() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_allows_simple_destructure.ds",
            r#"
const { x } = obj
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_detects_useless_rename_in_function_param() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_detects_useless_rename_in_function_param.ds",
            r#"
function foo({ a: a }) {}
"#,
        );
        test.result(result).assert_lint("no-useless-rename");
    }

    #[test]
    fn test_fix_useless_rename() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_fix_useless_rename.ds",
            r#"
const { x: x } = obj;
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_safe_fixed(
                r#"
const { x } = obj;
"#,
            );
    }

    #[test]
    fn test_no_fix_when_field_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_no_fix_when_field_contains_comment.ds",
            r#"
const { x /* keep */: x } = obj;
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_has_no_fix("no-useless-rename");
    }

    #[test]
    fn test_allows_string_key_alias_with_same_identifier() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_allows_string_key_alias_with_same_identifier.ds",
            r#"
const { "x": x } = obj;
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_fix_preserves_default_value_in_destructure() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_fix_preserves_default_value_in_destructure.ds",
            r#"
const { value: value = 1 } = obj;
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_safe_fixed(
                r#"
const { value = 1 } = obj;
"#,
            );
    }

    #[test]
    fn test_detects_useless_import_rename() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_detects_useless_import_rename.ds",
            r#"
import { value as value } from "./source.ds";
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_safe_fixed(
                r#"
import { value } from "./source.ds";
"#,
            );
    }

    #[test]
    fn test_detects_useless_export_rename() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename);
        let result = test.lint_ast(
            "no_useless_rename/test_detects_useless_export_rename.ds",
            r#"
export { value as value } from "./source.ds";
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_safe_fixed(
                r#"
export { value } from "./source.ds";
"#,
            );
    }

    #[test]
    fn test_allows_destructuring_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename).with_options(|options| {
            options.correctness.no_useless_rename_ignore_destructuring = true;
        });
        let result = test.lint_ast(
            "no_useless_rename/test_allows_destructuring_when_ignored.ds",
            r#"
const { x: x } = obj;
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_import_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename).with_options(|options| {
            options.correctness.no_useless_rename_ignore_import = true;
        });
        let result = test.lint_ast(
            "no_useless_rename/test_allows_import_when_ignored.ds",
            r#"
import { value as value } from "./source.ds";
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_export_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoUselessRename).with_options(|options| {
            options.correctness.no_useless_rename_ignore_export = true;
        });
        let result = test.lint_ast(
            "no_useless_rename/test_allows_export_when_ignored.ds",
            r#"
export { value as value } from "./source.ds";
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }
}
