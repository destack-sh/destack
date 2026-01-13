use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit const on borrows and raw pointers.
    ///
    /// `&T` and `*T` are already immutable, so `&const T` and `*const T`
    /// are redundant and noisy.
    #[lint(
        id = "redundant-const-reference",
        code = "LY082",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub RedundantConstReference,
    "Disallow redundant const on references and raw pointers"
}

impl LintRule for RedundantConstReference {
    fn meta(&self) -> &'static crate::LintMeta {
        RedundantConstReference::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // lint metadata
        let meta = self.meta();

        // scan expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // load expression
            let expression = ctx.tree.get(node_id);

            // detect const reference form
            let marker = match expression {
                ast::Expression::ReferenceOf {
                    mutability: Some(ast::Mutability::Immutable),
                    ..
                } => '&',
                ast::Expression::PointerOf {
                    mutability: Some(ast::Mutability::Immutable),
                    ..
                } => '*',
                _ => continue,
            };

            // resolve severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build diagnostic
            let span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                REDUNDANT_CONST_REFERENCE.id,
                REDUNDANT_CONST_REFERENCE.code,
                REDUNDANT_CONST_REFERENCE.category,
                severity,
                "redundant const on reference or raw pointer",
                ctx.module.file_id,
                span,
            )
            .with_label("use &T or *T without const");

            // attach fix when possible
            if ctx.compute_fixes
                && let Some(replacement) = strip_const_modifier(ctx.get_span_text(span), marker)
            {
                let edits = ctx.edit_builder().replace(span, replacement).into_edits();
                let fix = LintFix::safe("Remove redundant const").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            // emit report
            ctx.report(diagnostic);
        }
    }
}

fn strip_const_modifier(text: &str, marker: char) -> Option<String> {
    // check leading marker
    let mut chars = text.char_indices();
    let (_, first) = chars.next()?;
    if first != marker {
        return None;
    }

    // skip trivia after marker
    let mut index = 1;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        // skip whitespace
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        // skip line comments
        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'/' {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }

        // skip block comments
        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            if index + 1 < bytes.len() {
                index += 2;
                continue;
            }
            return None;
        }

        break;
    }

    // match const keyword
    let rest = &text[index..];
    if !rest.starts_with("const") {
        return None;
    }

    // reject const identifiers
    let after_index = index + "const".len();
    if let Some(next) = text[after_index..].chars().next()
        && (next.is_ascii_alphanumeric() || next == '_')
    {
        return None;
    }

    // skip trailing whitespace
    let mut suffix_index = after_index;
    while suffix_index < bytes.len() && bytes[suffix_index].is_ascii_whitespace() {
        suffix_index += 1;
    }

    // build replacement
    let mut result = String::with_capacity(text.len().saturating_sub("const".len()));
    result.push(marker);
    result.push_str(&text[1..index]);
    result.push_str(&text[suffix_index..]);
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report redundant const on borrow types.
    #[test]
    fn test_reports_redundant_const_borrow() {
        let test = TestProgram::for_rule_without_prelude(RedundantConstReference);
        let result = test.lint_ast(
            "test.ds",
            r#"
type T = &const Foo
"#,
        );

        test.result(result)
            .assert_lint("redundant-const-reference")
            .assert_safe_fixed(
                r#"
type T = &Foo
"#,
            );
    }

    /// Report redundant const on pointer types.
    #[test]
    fn test_reports_redundant_const_pointer() {
        let test = TestProgram::for_rule_without_prelude(RedundantConstReference);
        let result = test.lint_ast(
            "test.ds",
            r#"
type T = *const Foo
"#,
        );

        test.result(result)
            .assert_lint("redundant-const-reference")
            .assert_safe_fixed(
                r#"
type T = *Foo
"#,
            );
    }

    /// Allow borrows without explicit const.
    #[test]
    fn test_allows_plain_borrow() {
        let test = TestProgram::for_rule_without_prelude(RedundantConstReference);
        let result = test.lint_ast(
            "test.ds",
            r#"
type T = &Foo
"#,
        );

        test.result(result)
            .assert_no_lint("redundant-const-reference");
    }

    /// Allow pointers without explicit const.
    #[test]
    fn test_allows_plain_pointer() {
        let test = TestProgram::for_rule_without_prelude(RedundantConstReference);
        let result = test.lint_ast(
            "test.ds",
            r#"
type T = *Foo
"#,
        );

        test.result(result)
            .assert_no_lint("redundant-const-reference");
    }

    /// Allow mutable borrows and pointers.
    #[test]
    fn test_allows_mutable_reference_forms() {
        let test = TestProgram::for_rule_without_prelude(RedundantConstReference);
        let result = test.lint_ast(
            "test.ds",
            r#"
type T = &mut Foo
type U = *mut Foo
"#,
        );

        test.result(result)
            .assert_no_lint("redundant-const-reference");
    }
}
