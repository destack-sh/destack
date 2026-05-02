use std::collections::HashSet;

use destack_ast::{self as ast, Declaration, DependencyMode, ExportMode, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_path_segments, expression_unwrap_parenthesized_source_form};
use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow anonymous default exports.
    ///
    /// Anonymous default exports make it harder to search for usages and
    /// can lead to inconsistent naming across imports. Named exports are
    /// preferred for better discoverability and refactoring support.
    #[lint(
        id = "no-anonymous-default-export",
        code = "LR002",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoAnonymousDefaultExport,
    "Disallow anonymous default exports"
}

impl LintRule for NoAnonymousDefaultExport {
    fn meta(&self) -> &'static LintMeta {
        NoAnonymousDefaultExport::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // check anonymous default export declarations (e.g., `export default function() {}`)
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            // check if this is a default anonymous declaration
            let declaration = ctx.tree.get(declaration_id);
            let (is_default_export, is_anonymous) = match declaration {
                Declaration::Function(declaration) => (
                    declaration.export == Some(ExportMode::Default),
                    declaration.name.is_none(),
                ),
                Declaration::Class(declaration) => (
                    declaration.export == Some(ExportMode::Default),
                    declaration.name.is_none(),
                ),
                _ => continue,
            };

            // keep only default anonymous declarations
            if !is_default_export || !is_anonymous {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, declaration_id);

            // skip disabled diagnostics
            if !severity.is_enabled() {
                continue;
            }

            // build declaration diagnostic
            let span = ctx.tree.get_span(declaration_id);
            let mut diagnostic = LintReport::new(
                NO_ANONYMOUS_DEFAULT_EXPORT.id,
                NO_ANONYMOUS_DEFAULT_EXPORT.code,
                NO_ANONYMOUS_DEFAULT_EXPORT.category,
                severity,
                "anonymous default export",
                span,
            )
            .label("assign a name to this export");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) =
                    anonymous_default_declaration_fix(ctx, declaration_id, declaration)
            {
                diagnostic = diagnostic.fix(fix);
            }

            // report declaration diagnostic
            ctx.report(diagnostic);
        }

        // check anonymous default export expressions (e.g., `export default { foo: 1 }`)
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // keep only export expressions
            let expression = ctx.tree.get(expression_id);
            let Expression::Export { items, .. } = expression else {
                continue;
            };

            // check each item in the export
            for item_id in items {
                // check if this is a default anonymous value export
                let item = ctx.tree.get(*item_id);

                // keep only default export items with values
                let ast::DependencyItem::Item {
                    mode: DependencyMode::Default,
                    value: Some(item_value),
                    ..
                } = item
                else {
                    continue;
                };
                let value_id = expression_unwrap_parenthesized_source_form(ctx.tree, *item_value);
                let value = ctx.tree.get(value_id);

                // skip named or call expression exports
                if expression_path_segments(ctx.tree, value_id).is_some()
                    || matches!(value, Expression::Call { .. })
                {
                    continue;
                }

                // resolve effective lint severity
                let severity = ctx.get_effective_severity(meta, expression_id);

                // skip disabled diagnostics
                if !severity.is_enabled() {
                    continue;
                }

                // report expression diagnostic
                let span = ctx.tree.get_span(expression_id);
                ctx.report(
                    LintReport::new(
                        NO_ANONYMOUS_DEFAULT_EXPORT.id,
                        NO_ANONYMOUS_DEFAULT_EXPORT.code,
                        NO_ANONYMOUS_DEFAULT_EXPORT.category,
                        severity,
                        "anonymous default export",
                        span,
                    )
                    .label("assign a name to this export"),
                );
            }
        }
    }
}

/// Build an unsafe fix for anonymous default declaration exports.
fn anonymous_default_declaration_fix(
    ctx: &LintAstContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
    declaration: &Declaration,
) -> Option<LintFix> {
    // map declaration kind to rewrite strategy
    let kind = match declaration {
        Declaration::Function(_) => DefaultDeclarationKind::Function,
        Declaration::Class(_) => DefaultDeclarationKind::Class,
        _ => return None,
    };

    // build replacement text with an inserted default name
    let declaration_span = ctx.tree.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);
    let replacement_name = fresh_export_name(ctx, "defaultExport");
    let insert_offset = kind.default_name_insert_offset(declaration_text)?;
    let replacement = insert_text(
        declaration_text,
        insert_offset,
        &format!(" {replacement_name} "),
    );
    let edits = ctx
        .edit_builder()
        .replace(declaration_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Add explicit name to anonymous default export").with_edits(edits))
}

/// The declaration kind for anonymous default export rewrites.
enum DefaultDeclarationKind {
    Function,
    Class,
}

impl DefaultDeclarationKind {
    /// Resolve the text insertion offset for adding `defaultExport`.
    fn default_name_insert_offset(&self, declaration_text: &str) -> Option<usize> {
        // delegate to declaration kind-specific offset logic
        match self {
            Self::Function => function_name_insert_offset(declaration_text),
            Self::Class => class_name_insert_offset(declaration_text),
        }
    }
}

/// Resolve insertion offset for anonymous function declarations.
fn function_name_insert_offset(declaration_text: &str) -> Option<usize> {
    let keyword_offset = declaration_text.find("function")?;
    let mut cursor = keyword_offset + "function".len();
    let bytes = declaration_text.as_bytes();

    // handle generator form: function* ()
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    // skip one generator marker
    if cursor < bytes.len() && bytes[cursor] == b'*' {
        cursor += 1;
    }

    // skip leading whitespace before the export token
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    Some(cursor)
}

/// Resolve insertion offset for anonymous class declarations.
fn class_name_insert_offset(declaration_text: &str) -> Option<usize> {
    let keyword_offset = declaration_text.find("class")?;
    let mut cursor = keyword_offset + "class".len();
    let bytes = declaration_text.as_bytes();

    // skip whitespace between `class` and an optional identifier
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    Some(cursor)
}

/// Insert text at a byte offset.
fn insert_text(text: &str, offset: usize, insertion: &str) -> String {
    let mut rewritten = String::new();
    rewritten.push_str(&text[..offset]);
    rewritten.push_str(insertion);
    rewritten.push_str(&text[offset..]);
    rewritten
}

/// Return a fresh identifier name that does not collide with existing bound names.
fn fresh_export_name(ctx: &LintAstContext<'_>, base_name: &str) -> String {
    let occupied_names = collect_occupied_names(ctx);
    let mut candidate = base_name.to_string();

    while occupied_names.contains(candidate.as_str()) {
        candidate.push('_');
    }

    candidate
}

/// Collect occupied binding names visible in the current module.
fn collect_occupied_names(ctx: &LintAstContext<'_>) -> HashSet<String> {
    let mut names = HashSet::new();

    for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
        if let Some(name) = ctx.tree.get(declaration_id).name() {
            names.insert(ctx.strings.get(name.string()).to_string());
        }
    }

    for pattern_id in ctx.tree.iter_nodes::<ast::Pattern>() {
        let ast::Pattern::Binding { name, .. } = ctx.tree.get(pattern_id) else {
            continue;
        };
        names.insert(ctx.strings.get(*name).to_string());
    }

    for parameter_id in ctx.tree.iter_nodes::<ast::Parameter>() {
        match ctx.tree.get(parameter_id) {
            ast::Parameter::Named { name, .. } | ast::Parameter::VariadicNamed { name, .. } => {
                names.insert(ctx.strings.get(*name).to_string());
            }
            ast::Parameter::Pattern { .. }
            | ast::Parameter::VariadicPattern { .. }
            | ast::Parameter::Error => {}
        }
    }

    for pattern_field_id in ctx.tree.iter_nodes::<ast::PatternField>() {
        match ctx.tree.get(pattern_field_id) {
            ast::PatternField::Named { name, pattern, .. } => {
                if pattern.is_none() {
                    names.insert(ctx.strings.get(name.string()).to_string());
                } else if let Some(pattern_id) = pattern
                    && let Some(binding_name) = named_pattern_field_binding_name(ctx, *pattern_id)
                {
                    names.insert(ctx.strings.get(binding_name).to_string());
                }
            }
            _ => {}
        }
    }

    for item_id in ctx.tree.iter_nodes::<ast::DependencyItem>() {
        let ast::DependencyItem::Item { name, alias, .. } = ctx.tree.get(item_id) else {
            continue;
        };
        if let Some(name) = name {
            names.insert(ctx.strings.get(name.string()).to_string());
        }
        if let Some(alias) = alias {
            names.insert(ctx.strings.get(*alias).to_string());
        }
    }

    names
}

/// Return the binding name introduced by one nested named field pattern.
fn named_pattern_field_binding_name(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::StringId> {
    match ctx.tree.get(pattern_id) {
        ast::Pattern::Binding { name, .. } => Some(*name),
        ast::Pattern::Assign { pattern, .. } => named_pattern_field_binding_name(ctx, *pattern),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_anonymous_function() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_anonymous_function.ts",
            r#"
export default function() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_has_fix("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_anonymous_class() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_anonymous_class.ts",
            r#"
export default class {
    foo() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_has_fix("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_object_literal() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_object_literal.ts",
            r#"
export default { foo: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_literal() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_literal.ts",
            r#"
export default 42
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_arrow_function() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_arrow_function.ts",
            r#"
export default (x) => x * 2
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_array_literal() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_array_literal.ts",
            r#"
export default [1, 2, 3];
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_new_expression() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_detects_new_expression.ts",
            r#"
export default new Value();
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_call_expression_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_allows_call_expression_by_default.ts",
            r#"
export default makeValue();
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_function() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_allows_named_function.ts",
            r#"
export default function myFunction() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_class() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_allows_named_class.ts",
            r#"
export default class MyClass {
    foo() {}
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_identifier_export() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_allows_identifier_export.ts",
            r#"
const myValue = 42;
export default myValue;
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_exports() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_allows_named_exports.ts",
            r#"
export const foo = 1;
export function bar() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_fix_renames_anonymous_default_function() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_fix_renames_anonymous_default_function.ts",
            r#"
export default function() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_unsafe_fixed(
                r#"
export default function defaultExport() {
    return 42;
}
"#,
            );
    }

    #[test]
    fn test_fix_renames_anonymous_default_class() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_fix_renames_anonymous_default_class.ts",
            r#"
export default class {
    foo() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_unsafe_fixed(
                r#"
export default class defaultExport {
    foo() {}
}
"#,
            );
    }

    #[test]
    fn test_fix_uses_fresh_name_when_default_export_is_taken() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_fix_uses_fresh_name_when_default_export_is_taken.ts",
            r#"
const defaultExport = 1;
export default function() {
    return defaultExport;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_unsafe_fixed(
                r#"
const defaultExport = 1;
export default function defaultExport_() {
    return defaultExport;
}
"#,
            );
    }

    #[test]
    fn test_fix_uses_fresh_name_when_pattern_binding_is_taken() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_fix_uses_fresh_name_when_pattern_binding_is_taken.ts",
            r#"
const [defaultExport] = values;
export default function() {
    return defaultExport;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_unsafe_fixed(
                r#"
const [defaultExport] = values;
export default function defaultExport_() {
    return defaultExport;
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_renames_anonymous_default_generator_function() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_mutation_fix_renames_anonymous_default_generator_function.ts",
            r#"
export default function*() {
    yield 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_unsafe_fixed(
                r#"
export default function* defaultExport() {
    yield 1;
}
"#,
            );
    }

    #[test]
    fn test_no_fix_for_anonymous_default_expression_export() {
        let test = TestProgram::for_rule_without_prelude(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "no_anonymous_default_export/test_no_fix_for_anonymous_default_expression_export.ts",
            r#"
export default { foo: 1 };
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export")
            .assert_has_no_fix("no-anonymous-default-export");
    }
}
