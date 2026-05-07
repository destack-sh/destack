use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelTargetKind {
    Loop,
    Switch,
    Other,
}

declare_lint! {
    /// Disallow labeled statements.
    ///
    /// Labeled statements are rarely needed and can make control flow harder
    /// to understand. Consider restructuring with functions or different loop patterns.
    #[lint(
        id = "no-labels",
        code = "LR015",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoLabels,
    "Disallow labeled statements"
}

impl LintRule for NoLabels {
    fn meta(&self) -> &'static LintMeta {
        NoLabels::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let (message, label) = match expression {
                ast::Expression::Labelled { .. }
                    if !label_kind_is_allowed(label_target_kind(ctx, node_id), ctx) =>
                {
                    ("labeled statement is not allowed", "avoid using labels")
                }
                ast::Expression::Break {
                    label: Some(label_name),
                    ..
                } if !label_kind_is_allowed(
                    referenced_label_target_kind(ctx, node_id, *label_name),
                    ctx,
                ) =>
                {
                    (
                        "label in break statement is not allowed",
                        "remove the label from this break statement",
                    )
                }
                ast::Expression::Continue {
                    label: Some(label_name),
                } if !label_kind_is_allowed(
                    referenced_label_target_kind(ctx, node_id, *label_name),
                    ctx,
                ) =>
                {
                    (
                        "label in continue statement is not allowed",
                        "remove the label from this continue statement",
                    )
                }
                _ => continue,
            };

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_LABELS.id,
                    NO_LABELS.code,
                    NO_LABELS.category,
                    severity,
                    message,
                    span,
                )
                .label(label),
            );
        }
    }
}

/// Return the kind of one labeled expression body.
fn label_target_kind(
    ctx: &LintAstContext<'_>,
    labelled_expression_id: ast::LocalNodeId<ast::Expression>,
) -> LabelTargetKind {
    let ast::Expression::Labelled { body, .. } = ctx.tree.get(labelled_expression_id) else {
        return LabelTargetKind::Other;
    };

    match ctx.tree.get(*body) {
        ast::Expression::While { .. }
        | ast::Expression::ForEach { .. }
        | ast::Expression::For { .. }
        | ast::Expression::Loop { .. } => LabelTargetKind::Loop,
        ast::Expression::Match {
            form: ast::MatchForm::Switch,
            ..
        } => LabelTargetKind::Switch,
        _ => LabelTargetKind::Other,
    }
}

/// Return the kind of one referenced label target.
fn referenced_label_target_kind(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    label_name: ast::StringId,
) -> LabelTargetKind {
    let mut current = Some(expression_id.id);

    while let Some(node_id) = current {
        let parent_id = ctx.parents.get_by_id(node_id);
        let Some(parent_id) = parent_id else {
            return LabelTargetKind::Other;
        };
        current = Some(parent_id);

        if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
            continue;
        }

        let labelled_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
        let ast::Expression::Labelled { label, .. } = ctx.tree.get(labelled_expression_id) else {
            continue;
        };
        if *label == label_name {
            return label_target_kind(ctx, labelled_expression_id);
        }
    }

    LabelTargetKind::Other
}

/// Return true when one label target kind is allowed by configuration.
fn label_kind_is_allowed(kind: LabelTargetKind, ctx: &LintAstContext<'_>) -> bool {
    match kind {
        LabelTargetKind::Loop => ctx.options.restriction.allow_loop_labels,
        LabelTargetKind::Switch => ctx.options.restriction.allow_switch_labels,
        LabelTargetKind::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_labeled_statement() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_labeled_statement.ts",
            r#"
outer: for (let i = 0; i < 10; i++) {
    break outer;
}
"#,
        );
        test.result(result).assert_lint("no-labels");
    }

    #[test]
    fn test_allows_unlabeled_loop() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_allows_unlabeled_loop.ts",
            r#"
for (let i = 0; i < 10; i++) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_detects_break_with_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_break_with_label.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    break outer;
}
"#,
        );
        test.result(result).assert_lint_count("no-labels", 2);
    }

    #[test]
    fn test_detects_continue_with_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_detects_continue_with_label.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    continue outer;
}
"#,
        );
        test.result(result).assert_lint_count("no-labels", 2);
    }

    #[test]
    fn test_allows_break_without_label() {
        let test = TestProgram::for_rule_without_prelude(NoLabels);
        let result = test.lint_ast(
            "no_labels/test_allows_break_without_label.ts",
            r#"
for (let i = 0; i < 2; i++) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_allows_loop_labels_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoLabels).with_options(|options| {
            options.restriction.allow_loop_labels = true;
        });
        let result = test.lint_ast(
            "no_labels/test_allows_loop_labels_when_enabled.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    break outer;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_allows_continue_loop_labels_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoLabels).with_options(|options| {
            options.restriction.allow_loop_labels = true;
        });
        let result = test.lint_ast(
            "no_labels/test_allows_continue_loop_labels_when_enabled.ts",
            r#"
outer: for (let i = 0; i < 2; i++) {
    continue outer;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_allows_switch_labels_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoLabels).with_options(|options| {
            options.restriction.allow_switch_labels = true;
        });
        let result = test.lint_ast(
            "no_labels/test_allows_switch_labels_when_enabled.ts",
            r#"
outer: switch (value) {
    case 1:
        break outer;
    default:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-labels");
    }

    #[test]
    fn test_still_detects_non_loop_labels_when_loop_labels_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoLabels).with_options(|options| {
            options.restriction.allow_loop_labels = true;
        });
        let result = test.lint_ast(
            "no_labels/test_still_detects_non_loop_labels_when_loop_labels_enabled.ts",
            r#"
outer: {
    break outer;
}
"#,
        );
        test.result(result).assert_lint_count("no-labels", 2);
    }
}
