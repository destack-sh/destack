use std::collections::HashSet;

use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate case labels in switch statements.
    ///
    /// Having duplicate case labels in a switch statement is almost always a mistake.
    /// Only the first matching case will be executed.
    #[lint(
        id = "no-duplicate-case",
        code = "LC009",
        category = Correctness,
        level = Ast
    )]
    pub NoDuplicateCase,
    "Disallow duplicate case labels"
}

impl LintRule for NoDuplicateCase {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateCase::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { kind, cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // only check switch statements, not match expressions
            if *kind != ast::MatchKind::Switch {
                continue;
            }

            // collect case patterns and check for duplicates
            let mut seen_literals: HashSet<LiteralKey> = HashSet::new();

            for case_id in cases {
                let case = ctx.tree.get(*case_id);
                let pattern_id = match case {
                    ast::MatchCase::Expression { pattern, .. } => pattern,
                    ast::MatchCase::Block { pattern, .. } => pattern,
                };

                let pattern = ctx.tree.get(*pattern_id);
                if let Some(literal_key) = pattern_to_literal_key(ctx, pattern) {
                    if seen_literals.contains(&literal_key) {
                        ctx.report(
                            LintDiagnostic::new(
                                NO_DUPLICATE_CASE.id,
                                NO_DUPLICATE_CASE.code,
                                NO_DUPLICATE_CASE.category,
                                severity,
                                "duplicate case label",
                                ctx.module.file_id,
                                ctx.tree.get_span(*case_id),
                            )
                            .with_label("this case was already handled"),
                        );
                    } else {
                        seen_literals.insert(literal_key);
                    }
                }
            }
        }
    }
}

/// A hashable key representing a literal value for comparison.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum LiteralKey {
    Boolean(bool),
    Integer(i64),
    String(String),
    Null,
    Undefined,
}

/// Convert a pattern to a literal key for duplicate detection.
fn pattern_to_literal_key(
    ctx: &LintModuleAstContext<'_>,
    pattern: &ast::Pattern,
) -> Option<LiteralKey> {
    match pattern {
        ast::Pattern::Expression { value } => {
            let expression = ctx.tree.get(*value);
            expression_to_literal_key(ctx, expression)
        }
        _ => None,
    }
}

/// Convert an expression to a literal key.
fn expression_to_literal_key(
    ctx: &LintModuleAstContext<'_>,
    expression: &ast::Expression,
) -> Option<LiteralKey> {
    match expression {
        ast::Expression::ScalarLiteral(literal) => match literal {
            ast::ScalarLiteral::Boolean(b) => Some(LiteralKey::Boolean(*b)),
            ast::ScalarLiteral::Integer(i) => Some(LiteralKey::Integer(*i)),
            ast::ScalarLiteral::String(s) => {
                Some(LiteralKey::String(ctx.strings.get(*s).to_string()))
            }
            _ => None,
        },
        ast::Expression::TypeLiteral(literal) => match literal {
            ast::TypeLiteral::Null => Some(LiteralKey::Null),
            ast::TypeLiteral::Undefined => Some(LiteralKey::Undefined),
            _ => None,
        },
        ast::Expression::Parenthesized { expression } => {
            let inner = ctx.tree.get(*expression);
            expression_to_literal_key(ctx, inner)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_integer_case() {
        let test = TestProgram::for_rule(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 1: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_detects_duplicate_string_case() {
        let test = TestProgram::for_rule(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = "a";
switch (x) {
    case "a": break;
    case "b": break;
    case "a": break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_detects_duplicate_boolean_case() {
        let test = TestProgram::for_rule(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = true;
switch (x) {
    case true: break;
    case false: break;
    case true: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_allows_unique_cases() {
        let test = TestProgram::for_rule(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 3: break;
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }

    #[test]
    fn test_ignores_match_expression() {
        let test = TestProgram::for_rule(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
match (x) {
    1 => 1
    2 => 2
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }
}
