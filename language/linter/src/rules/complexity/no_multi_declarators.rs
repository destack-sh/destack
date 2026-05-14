use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use destack_source::Span;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow multiple declarators in a single let/const statement.
    ///
    /// Multiple declarators in a single statement like `let a = 1, b = 2` can be harder to read and maintain.
    /// Each binding should be its own statement for better clarity and easier modification.
    #[lint(
        id = "no-multi-declarators",
        code = "LX020",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoMultiDeclarators,
    "Disallow multiple declarators in let/const statements"
}

impl LintRule for NoMultiDeclarators {
    fn meta(&self) -> &'static LintMeta {
        NoMultiDeclarators::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for expression_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let (declarators, let_kind) = match ctx.dir.get(expression_id) {
                dir::Expression::Let {
                    declarators, kind, ..
                } => (declarators.as_slice(), Some(*kind)),
                dir::Expression::Using { declarators, .. } => (declarators.as_slice(), None),
                _ => continue,
            };

            // check if there are multiple declarators
            if declarators.len() > 1 {
                // skip for-loop initializers
                if is_for_initializer(ctx, expression_id) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, expression_id);
                if !severity.is_enabled() {
                    continue;
                }

                let span = ctx.dir.get_span(expression_id);
                let mut diagnostic = LintReport::new(
                    NO_MULTI_DECLARATORS.id,
                    NO_MULTI_DECLARATORS.code,
                    NO_MULTI_DECLARATORS.category,
                    severity,
                    "multiple declarators in single statement",
                    span,
                )
                .label("split into separate statements");

                if let Some(kind) = let_kind
                    && let Some(fix) = split_declarator_fix(ctx, expression_id, declarators, kind)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build a fix that splits one let statement into one statement per declarator.
fn split_declarator_fix(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    declarators: &[dir::LocalNodeId<dir::Declarator>],
    kind: dir::LetKind,
) -> Option<LintFix> {
    // skip for-loop initializers: splitting requires control-flow surgery
    if is_for_initializer(ctx, expression_id) {
        return None;
    }

    // skip comment-bearing declarations to avoid dropping trivia
    let statement_span = ctx.dir.get_span(expression_id);
    let statement_text = ctx.get_span_text(statement_span);
    if statement_text.contains("//") || statement_text.contains("/*") {
        return None;
    }

    // derive prefix text up to the first declarator
    let first_declarator_span = ctx.dir.get_span(declarators[0]);
    let prefix_span = Span::new(
        statement_span.file,
        statement_span.start,
        first_declarator_span.start,
    );
    let mut prefix = ctx.get_span_text(prefix_span).to_string();
    if prefix.trim().is_empty() {
        let keyword = match kind {
            dir::LetKind::Let => "let ",
            dir::LetKind::Const => "const ",
        };
        prefix = keyword.to_string();
    }

    // build one statement per declarator
    let mut statements = Vec::new();
    for declarator_id in declarators {
        let declarator_span = ctx.dir.get_span(*declarator_id);
        let declarator_text = ctx.get_span_text(declarator_span);
        statements.push(format!("{prefix}{declarator_text};"));
    }

    // replace the original multi declarator statement
    let replacement = statements.join("\n");
    let edits = ctx
        .edit_builder()
        .replace(statement_span, replacement)
        .into_edits();

    Some(LintFix::safe("Split declarators into separate statements").with_edits(edits))
}

/// Return true when one expression is the initializer of a for loop.
fn is_for_initializer(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent_id) = ctx.dir.get_parent_id(expression_id.id) else {
        return false;
    };
    if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
        return false;
    }

    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = ctx.dir.get(parent_expression_id);
    matches!(
        parent_expression,
        dir::Expression::For {
            initialization: Some(initialization_id),
            ..
        } if *initialization_id == expression_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_multiple_declarators_with_let() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_detects_multiple_declarators_with_let.ds",
            r#"
let a = 1, b = 2;
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_has_fix("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_with_const() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_detects_multiple_declarators_with_const.ds",
            r#"
const a = 1, b = 2, c = 3;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_with_var() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_detects_multiple_declarators_with_var.ds",
            r#"
let x = 1, y = 2;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_without_values() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_detects_multiple_declarators_without_values.ds",
            r#"
let a: int32, b: int32;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_fix_multiple_declarators_with_const() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_fix_multiple_declarators_with_const.ds",
            r#"
const a = 1, b = 2, c = 3;
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_has_fix("no-multi-declarators")
            .assert_safe_fixed(
                r#"
const a = 1;
const b = 2;
const c = 3;
"#,
            );
    }

    #[test]
    fn test_fix_multiple_declarators_with_type_annotations() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_fix_multiple_declarators_with_type_annotations.ds",
            r#"
let a: int32 = 1, b: int32 = 2;
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_safe_fixed(
                r#"
let a: int32 = 1;
let b: int32 = 2;
"#,
            );
    }

    #[test]
    fn test_allows_multi_declarators_in_for_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_allows_multi_declarators_in_for_initializer.ds",
            r#"
for (let a = 0, b = 1; a < 10; a++) {}
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_no_fix_for_comment_bearing_statement() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_no_fix_for_comment_bearing_statement.ds",
            r#"
let a = 1, /* keep */ b = 2;
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_has_no_fix("no-multi-declarators");
    }

    #[test]
    fn test_mutation_fix_with_destructuring_and_binding() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_mutation_fix_with_destructuring_and_binding.ds",
            r#"
let (x, y) = point, z = 0;
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_has_fix("no-multi-declarators")
            .assert_safe_fixed(
                r#"
let (x, y) = point;
let z = 0;
"#,
            );
    }

    #[test]
    fn test_reports_using_multi_declarators_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_reports_using_multi_declarators_without_fix.ds",
            r#"
using a = openA(), b = openB();
"#,
        );
        test.result(result)
            .assert_lint("no-multi-declarators")
            .assert_has_no_fix("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_with_let() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_allows_single_declarator_with_let.ds",
            r#"
let a = 1;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_with_const() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_allows_single_declarator_with_const.ds",
            r#"
const x = 42;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_multiple_separate_statements() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_allows_multiple_separate_statements.ds",
            r#"
let a = 1;
let b = 2;
let c = 3;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_without_value() {
        let test = TestProgram::for_rule_without_prelude(NoMultiDeclarators);
        let result = test.lint(
            "no_multi_declarators/test_allows_single_declarator_without_value.ds",
            r#"
let x: int32;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }
}
