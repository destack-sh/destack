use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::expression_statement_ancestor;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow sequence expressions (comma operator).
    ///
    /// The comma operator evaluates expressions left to right and returns the last value.
    /// This is confusing and error prone. Use separate statements instead.
    /// Note: In `.ds` files, `(a, b, c)` is a tuple literal, not a sequence expression.
    #[lint(
        id = "no-sequences",
        code = "LR025",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoSequences,
    "Disallow sequence expressions"
}

impl LintRule for NoSequences {
    fn meta(&self) -> &'static LintMeta {
        NoSequences::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            if !matches!(expression, dir::Expression::SequenceExpression { .. }) {
                continue;
            }
            if sequence_expression_is_for_part(ctx, node_id) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_SEQUENCES.id,
                NO_SEQUENCES.code,
                NO_SEQUENCES.category,
                severity,
                "sequence expression is not allowed",
                span,
            )
            .label("use separate statements instead of comma operator");
            if ctx.compute_fixes
                && let Some(fix) = no_sequences_fix(ctx, node_id, expression)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe fix for statement-level sequence expressions.
fn no_sequences_fix(
    ctx: &LintModuleContext<'_>,
    sequence_expression_id: dir::LocalNodeId<dir::Expression>,
    sequence_expression: &dir::Expression,
) -> Option<LintFix> {
    let dir::Expression::SequenceExpression { expressions } = sequence_expression else {
        return None;
    };
    if expressions.is_empty() {
        return None;
    }

    // keep statement position only
    let statement_expression_id =
        expression_statement_ancestor(ctx.dir.tree(), sequence_expression_id)?;

    // resolve statements
    let mut statements = Vec::new();
    for expression_id in expressions {
        let expression_text = ctx
            .get_span_text(ctx.dir.get_span(*expression_id))
            .trim()
            .to_string();
        if expression_text.is_empty() {
            return None;
        }
        statements.push(format!("{expression_text};"));
    }

    // build replacement text
    let replacement_text = statements.join("\n");
    let edits = ctx
        .edit_builder()
        .replace(ctx.dir.get_span(statement_expression_id), replacement_text)
        .into_edits();
    Some(LintFix::safe("Split sequence into separate statements").with_edits(edits))
}

/// Return true when one sequence expression appears in for init or increment.
fn sequence_expression_is_for_part(
    ctx: &LintModuleContext<'_>,
    sequence_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // start from the sequence expression
    let mut current_id = sequence_expression_id;

    // walk through parenthesized wrappers to one for init or increment boundary
    loop {
        let Some(parent_id) = ctx.dir.get_parent_id(current_id.id) else {
            return false;
        };
        if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
            return false;
        }

        // resolve parent expression id
        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = ctx.dir.get(parent_expression_id);
        match parent_expression {
            dir::Expression::Parenthesized { expression } if *expression == current_id => {
                current_id = parent_expression_id;
            }
            dir::Expression::For {
                initialization: Some(initialization_id),
                ..
            } if *initialization_id == current_id => {
                return true;
            }
            dir::Expression::For {
                increment: Some(increment_id),
                ..
            } if *increment_id == current_id => {
                return true;
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_sequence_expression() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_detects_sequence_expression.ts",
            r#"
let x = (1, 2, 3);
"#,
        );
        test.result(result).assert_lint("no-sequences");
    }

    #[test]
    fn test_allows_function_calls_with_multiple_args() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_allows_function_calls_with_multiple_args.ts",
            r#"
foo(1, 2, 3);
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_array_literals() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_allows_array_literals.ts",
            r#"
let arr = [1, 2, 3];
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_destack_tuples() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        // in .ds files, (1, 2) is a tuple, not a sequence
        let result = test.lint(
            "no_sequences/test_allows_destack_tuples.ds",
            r#"
let tuple = (1, 2, 3);
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_for_initialization_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_allows_for_initialization_sequence.ts",
            r#"
for ((first(), second()); ready(); step()) {}
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_for_increment_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_allows_for_increment_sequence.ts",
            r#"
for (; ready(); (stepA(), stepB())) {}
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_fix_splits_statement_sequence_expression() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_fix_splits_statement_sequence_expression.ts",
            r#"
(first(), second(), third());
"#,
        );
        test.result(result)
            .assert_lint("no-sequences")
            .assert_safe_fixed(
                r#"
first();
second();
third();
"#,
            );
    }

    #[test]
    fn test_fix_splits_parenthesized_statement_sequence_expression() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_fix_splits_parenthesized_statement_sequence_expression.ts",
            r#"
((first(), second(), third()));
"#,
        );
        test.result(result)
            .assert_lint("no-sequences")
            .assert_safe_fixed(
                r#"
first();
second();
third();
"#,
            );
    }

    #[test]
    fn test_allows_parenthesized_for_initialization_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_allows_parenthesized_for_initialization_sequence.ts",
            r#"
for (((first(), second())); ready(); step()) {}
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_no_fix_for_sequence_expression_used_as_value() {
        let test = TestProgram::for_rule_without_prelude(NoSequences);
        let result = test.lint(
            "no_sequences/test_no_fix_for_sequence_expression_used_as_value.ts",
            r#"
let value = (first(), second());
"#,
        );
        test.result(result)
            .assert_lint("no-sequences")
            .assert_has_no_fix("no-sequences");
    }
}
