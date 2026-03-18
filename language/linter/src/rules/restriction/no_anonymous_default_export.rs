use destack_ast::{self as ast, Declaration, DependencyMode, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized_syntax;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

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
            let (descriptor, is_anonymous) = match declaration {
                Declaration::Function { descriptor, .. } => (descriptor, descriptor.name.is_none()),
                Declaration::Class { descriptor, .. } => (descriptor, descriptor.name.is_none()),
                _ => continue,
            };

            // keep only default anonymous declarations
            if descriptor.export != Some(DependencyMode::Default) || !is_anonymous {
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
            let mut diagnostic = LintDiagnostic::new(
                NO_ANONYMOUS_DEFAULT_EXPORT.id,
                NO_ANONYMOUS_DEFAULT_EXPORT.code,
                NO_ANONYMOUS_DEFAULT_EXPORT.category,
                severity,
                "anonymous default export",
                ctx.module.file_id,
                span,
            )
            .with_label("assign a name to this export");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) =
                    anonymous_default_declaration_fix(ctx, declaration_id, declaration)
            {
                diagnostic = diagnostic.with_fix(fix);
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
                if item.mode != DependencyMode::Default || item.value.is_none() {
                    continue;
                }
                let Some(item_value) = item.value else {
                    continue;
                };
                let value_id = expression_unwrap_parenthesized_syntax(ctx.tree, item_value);
                let value = ctx.tree.get(value_id);

                // skip named or call expression exports
                if matches!(value, Expression::Path { .. } | Expression::Call { .. }) {
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
                    LintDiagnostic::new(
                        NO_ANONYMOUS_DEFAULT_EXPORT.id,
                        NO_ANONYMOUS_DEFAULT_EXPORT.code,
                        NO_ANONYMOUS_DEFAULT_EXPORT.category,
                        severity,
                        "anonymous default export",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("assign a name to this export"),
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
        Declaration::Function { .. } => DefaultDeclarationKind::Function,
        Declaration::Class { .. } => DefaultDeclarationKind::Class,
        _ => return None,
    };

    // build replacement text with an inserted default name
    let declaration_span = ctx.tree.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);
    let insert_offset = kind.default_name_insert_offset(declaration_text)?;
    let replacement = insert_text(declaration_text, insert_offset, " defaultExport ");
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
