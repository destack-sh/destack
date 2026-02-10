use destack_ast::{self as ast, DependencyKind, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type import style.
    ///
    /// Prefer `import type { Foo }` over `import { type Foo }` for type-only imports.
    /// This makes it clearer that the import is only used for type checking.
    #[lint(
        id = "consistent-type-imports",
        code = "LY007",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentTypeImports,
    "Enforce consistent type import style"
}

impl LintRule for ConsistentTypeImports {
    fn meta(&self) -> &'static crate::LintMeta {
        ConsistentTypeImports::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for import expressions
            let Expression::Import { kind, items, .. } = expr else {
                continue;
            };

            // if the import is already `import type`, skip
            if *kind == DependencyKind::Type {
                continue;
            }

            // check if ALL items are type imports
            let all_type_imports = !items.is_empty()
                && items.iter().all(|item_id| {
                    let item = ctx.tree.get(*item_id);
                    item.kind == Some(DependencyKind::Type)
                });

            if all_type_imports {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    CONSISTENT_TYPE_IMPORTS.id,
                    CONSISTENT_TYPE_IMPORTS.code,
                    CONSISTENT_TYPE_IMPORTS.category,
                    severity,
                    "use `import type { ... }` for type-only imports",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("prefer top-level `import type`");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = consistent_type_import_fix(ctx, node_id, items)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build one fix that rewrites inline type imports into top-level `import type`.
fn consistent_type_import_fix(
    ctx: &LintModuleAstContext<'_>,
    import_id: ast::LocalNodeId<ast::Expression>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> Option<LintFix> {
    let import_span = ctx.tree.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);
    let keyword_offset = import_text.find("import")?;

    // only auto-fix canonical import statements with leading trivia only
    if !import_text[..keyword_offset].trim().is_empty() {
        return None;
    }

    let mut builder = ctx.edit_builder().insert(
        import_span.start + keyword_offset as u32 + "import".len() as u32,
        " type",
    );

    // strip inline `type` prefixes from each import item
    for item_id in items {
        let item = ctx.tree.get(*item_id);
        if item.kind != Some(DependencyKind::Type) {
            continue;
        }

        let item_span = ctx.tree.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = strip_inline_type_keyword(item_text)?;
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to `import type` syntax").with_edits(edits))
}

/// Remove one leading `type` keyword from an import item text.
fn strip_inline_type_keyword(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    if !text[cursor..].starts_with("type") {
        return None;
    }

    let keyword_end = cursor + "type".len();
    if keyword_end >= bytes.len() || !bytes[keyword_end].is_ascii_whitespace() {
        return None;
    }

    let mut content_start = keyword_end;
    while content_start < bytes.len() && bytes[content_start].is_ascii_whitespace() {
        content_start += 1;
    }

    let mut rewritten = String::new();
    rewritten.push_str(&text[..cursor]);
    rewritten.push_str(&text[content_start..]);
    Some(rewritten)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_import_type() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_import_type.ds",
            r#"
import type { Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_detects_inline_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_detects_inline_type_imports.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_has_fix("consistent-type-imports");
    }

    #[test]
    fn test_allows_mixed_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        // mixed imports are allowed (can't use `import type` for these)
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_mixed_imports.ds",
            r#"
import { Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_value_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_value_imports.ds",
            r#"
import { foo, bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_namespace_import() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_namespace_import.ds",
            r#"
import * as foo from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_fix_rewrites_inline_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_fix_rewrites_inline_type_imports.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo, Bar } from "foo";
"#,
            );
    }

    #[test]
    fn test_fix_rewrites_alias_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_fix_rewrites_alias_type_imports.ds",
            r#"
import { type Foo as FooModel, type Bar as BarModel } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo as FooModel, Bar as BarModel } from "foo";
"#,
            );
    }

    #[test]
    fn test_mutation_detects_single_inline_type_import() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_mutation_detects_single_inline_type_import.ds",
            r#"
import { type OnlyType } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { OnlyType } from "foo";
"#,
            );
    }
}
