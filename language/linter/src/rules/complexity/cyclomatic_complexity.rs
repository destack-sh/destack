use crate::LintMeta;
use destack_dir::{
    self as dir, AssignOperator, BinaryOperator, Expression, FunctionSignature, LocalNodeId,
    MatchForm, NodeVisitor, NodeVisitorOptions, PatternField, Tree, walk_expression,
    walk_parameter, walk_pattern_field,
};
use destack_workspace::{CyclomaticComplexityVariant, LintSeverity};

use crate::rules::common::{
    expression_starts_nested_declaration_scope, parameter_default_expression_id,
    pattern_field_default_expression_id,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit cyclomatic complexity of functions.
    ///
    /// Cyclomatic complexity measures the number of linearly independent paths through a function.
    /// High complexity indicates code that is difficult to test and maintain.
    ///
    /// Complexity is incremented for each decision point:
    /// - `if`, `else if`, `while`, `for`, `loop`
    /// - Each `match` arm (except the first)
    /// - `catch` clauses
    /// - `&&`, `||`, and `??` operators
    /// - `?.` optional chaining
    /// - Ternary expressions
    #[lint(
        id = "cyclomatic-complexity",
        code = "LX002",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CyclomaticComplexity,
    "Limit cyclomatic complexity"
}

impl LintRule for CyclomaticComplexity {
    fn meta(&self) -> &'static LintMeta {
        CyclomaticComplexity::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve one shared max-threshold and lint meta
        let meta = self.meta();
        let max_complexity = ctx.options().complexity.max_cyclomatic_complexity;
        let variant = ctx.options().complexity.cyclomatic_complexity_variant;

        // check function declarations
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(body_id) = declaration.body else {
                continue;
            };

            report_body_complexity(
                ctx,
                meta,
                declaration_id,
                body_id,
                Some(&declaration.signature),
                max_complexity,
                variant,
            );
        }

        // check class field initializers as implicit function bodies
        for member_id in ctx.dir.iter_nodes::<dir::Member>() {
            let member = ctx.dir.get(member_id);
            let dir::Member::Field {
                default: Some(body_id),
                ..
            } = member
            else {
                continue;
            };

            report_body_complexity(
                ctx,
                meta,
                member_id,
                *body_id,
                None,
                max_complexity,
                variant,
            );
        }

        // check methods and static blocks
        for member_id in ctx.dir.iter_nodes::<dir::Member>() {
            let member = ctx.dir.get(member_id);
            let (body_id, signature) = match member {
                dir::Member::Method {
                    signature,
                    body: Some(body),
                    ..
                } => (*body, Some(signature)),
                dir::Member::StaticBlock { body, .. } => (*body, None),
                _ => continue,
            };

            report_body_complexity(
                ctx,
                meta,
                member_id,
                body_id,
                signature,
                max_complexity,
                variant,
            );
        }
    }
}

/// Report complexity when one callable body exceeds the configured maximum.
fn report_body_complexity<T: dir::Node>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: LocalNodeId<T>,
    body_expression_id: LocalNodeId<Expression>,
    signature: Option<&FunctionSignature>,
    max_complexity: usize,
    variant: CyclomaticComplexityVariant,
) {
    // calculate body complexity from one root expression
    let mut visitor = ComplexityVisitor {
        options: NodeVisitorOptions::default(),
        complexity: 1,
        root_expression_id: body_expression_id,
        variant,
    };

    // account for parameter defaults as assignment-pattern branches
    if let Some(signature) = signature {
        for parameter_id in &signature.parameters {
            let parameter = ctx.dir.get(*parameter_id);
            visitor.visit_parameter(ctx.dir.tree(), *parameter_id, parameter);
        }
    }

    let body_expression = ctx.dir.get(body_expression_id);
    visitor.visit_expression(ctx.dir.tree(), body_expression_id, body_expression);

    // skip bodies that are within configured threshold
    if visitor.complexity <= max_complexity {
        return;
    }

    // honor effective severity at the callable owner node
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one complexity overflow diagnostic
    ctx.report(
        LintReport::new(
            CYCLOMATIC_COMPLEXITY.id,
            CYCLOMATIC_COMPLEXITY.code,
            CYCLOMATIC_COMPLEXITY.category,
            severity,
            format!(
                "cyclomatic complexity {} exceeds maximum of {}",
                visitor.complexity, max_complexity
            ),
            ctx.dir.get_span(body_expression_id),
        )
        .label("consider breaking into smaller functions"),
    );
}

/// Visitor that calculates cyclomatic complexity for a function body.
struct ComplexityVisitor {
    /// Node visitor options.
    options: NodeVisitorOptions,
    /// Accumulated complexity score, starting at 1 (base complexity).
    complexity: usize,
    /// Root body expression for this callable.
    root_expression_id: LocalNodeId<Expression>,
    /// Switch counting variant.
    variant: CyclomaticComplexityVariant,
}

impl NodeVisitor for ComplexityVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // keep nested callable declarations scoped to their own complexity pass
        if id != self.root_expression_id && expression_starts_nested_declaration_scope(expression) {
            return;
        }

        // increment complexity for branching expressions
        match expression {
            Expression::If { .. } => {
                self.complexity += 1;
            }
            Expression::While { .. }
            | Expression::For { .. }
            | Expression::ForEach { .. }
            | Expression::Loop { .. } => {
                self.complexity += 1;
            }
            Expression::Match { form, cases, .. } => {
                self.complexity += match_case_complexity(tree, *form, cases, self.variant);
            }
            Expression::Try { catch, .. } => {
                if catch.is_some() {
                    self.complexity += 1;
                }
            }
            Expression::Binary { operator, .. } => {
                if matches!(
                    operator,
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
                ) {
                    self.complexity += 1;
                }
            }
            Expression::Assign { operator, .. } => {
                if matches!(
                    operator,
                    AssignOperator::AndAssign
                        | AssignOperator::OrAssign
                        | AssignOperator::CoalesceAssign
                ) {
                    self.complexity += 1;
                }
            }
            Expression::Maybe { .. } => {
                self.complexity += 1;
            }
            _ => {}
        }

        // recurse into expression children
        walk_expression(self, tree, id, expression);
    }

    fn visit_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        // defaulted parameters are assignment-pattern branches
        if parameter_default_expression_id(parameter).is_some() {
            self.complexity += 1;
        }

        // recurse into parameter children
        walk_parameter(self, tree, id, parameter);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        // defaulted destructuring fields are assignment-pattern branches
        if pattern_field_default_expression_id(tree, pattern_field).is_some() {
            self.complexity += 1;
        }

        // recurse into pattern-field children
        walk_pattern_field(self, tree, id, pattern_field);
    }
}

/// Return complexity increment from one match expression case set.
fn match_case_complexity(
    tree: &Tree,
    form: MatchForm,
    cases: &[LocalNodeId<dir::MatchCase>],
    variant: CyclomaticComplexityVariant,
) -> usize {
    // count switch cases according to the selected variant
    if form == MatchForm::Switch {
        if variant == CyclomaticComplexityVariant::Modified {
            return usize::from(!cases.is_empty());
        }

        return cases
            .iter()
            .copied()
            .filter(|case_id| !tree.get(*case_id).selector().is_default())
            .count();
    }

    // keep match-expression branch counting as a stable destack baseline
    cases.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_high_complexity() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 20);
        // create a function with 21+ decision points to exceed limit of 20
        let result = test.lint(
            "cyclomatic_complexity/test_detects_high_complexity.ds",
            r#"
function complex(a: bool) {
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
}
"#,
        );
        // 1 base + 21 if = 22 > 20
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_allows_simple_function() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity);
        let result = test.lint(
            "cyclomatic_complexity/test_allows_simple_function.ds",
            r#"
function simple(x: int32): int32 {
    if (x > 0) {
        return x
    }
    return -x
}
"#,
        );
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_logical_operators() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|opts| opts.complexity.max_cyclomatic_complexity = 20);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_logical_operators.ds",
            r#"
function manyConditions(a: bool): bool {
    return a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a
}
"#,
        );
        // 1 base + 21 && operators = 22 > 20
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_match_arms() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_match_arms.ds",
            r#"
function manyMatches(x: int32): string {
    match x {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        _ => "other"
    }
}
"#,
        );
        // 1 base + 5 extra arms = 6, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_ternary() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_ternary.ds",
            r#"
function nested(a: bool, b: bool): int32 {
    return a ? (b ? 1 : 2) : (b ? 3 : 4)
}
"#,
        );
        // 1 base + 4 ternaries = 5, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_try_catch() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_try_catch.ds",
            r#"
function withTry() {
    try {
        doSomething()
    } catch (error) {
        handleError(error)
    }
}
"#,
        );
        // 1 base + 1 catch = 2, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_logical_assignment_operators() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_logical_assignment_operators.ds",
            r#"
function logicalAssign(x: bool, y: bool): bool {
    x &&= y
    return x
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_default_parameter_as_branch() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_default_parameter_as_branch.ds",
            r#"
function withDefault(value = 1): int32 {
    return value
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_destructuring_default_as_branch() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_destructuring_default_as_branch.ds",
            r#"
function withPattern(value): int32 {
    let { count = 1 } = value
    return count
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_method_body_complexity() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_method_body_complexity.ds",
            r#"
class Demo {
    run(value: bool): int32 {
        if (value) {
            return 1
        }
        return 0
    }
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_does_not_count_nested_function_complexity_in_parent() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_does_not_count_nested_function_complexity_in_parent.ds",
            r#"
function outer(): int32 {
    function inner(value: bool): int32 {
        if (value) {
            return 1
        }
        return 0
    }
    return inner(false)
}
"#,
        );
        test.result(result)
            .assert_lint("cyclomatic-complexity")
            .assert_lint_count("cyclomatic-complexity", 1);
    }

    #[test]
    fn test_counts_switch_cases_without_default_case() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 3);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_switch_cases_without_default_case.ds",
            r#"
function withSwitch(value: int32): int32 {
    switch (value) {
        case 1: return 1;
        case 2: return 2;
        case 3: return 3;
    }
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_allows_modified_switch_complexity_variant() {
        let test =
            TestProgram::for_rule_without_prelude(CyclomaticComplexity).with_options(|options| {
                options.complexity.max_cyclomatic_complexity = 2;
                options.complexity.cyclomatic_complexity_variant =
                    CyclomaticComplexityVariant::Modified;
            });
        let result = test.lint(
            "cyclomatic_complexity/test_allows_modified_switch_complexity_variant.ds",
            r#"
function withSwitch(value: int32): int32 {
    switch (value) {
        case 1: return 1;
        case 2: return 2;
        case 3: return 3;
        default: return 0;
    }
}
"#,
        );
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_class_field_initializer_complexity() {
        let test = TestProgram::for_rule_without_prelude(CyclomaticComplexity)
            .with_options(|options| options.complexity.max_cyclomatic_complexity = 1);
        let result = test.lint(
            "cyclomatic_complexity/test_counts_class_field_initializer_complexity.ds",
            r#"
class Demo {
    value = if (flag) {
        1
    } else {
        2
    };
}
"#,
        );
        test.result(result).assert_lint("cyclomatic-complexity");
    }
}
