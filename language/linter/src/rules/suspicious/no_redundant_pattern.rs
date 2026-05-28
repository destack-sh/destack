use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow patterns that bind nothing useful.
    ///
    /// Patterns like `{ a: _ }` or `[_, _]` that only contain wildcards
    /// don't bind any values and are likely mistakes.
    #[lint(
        id = "no-redundant-pattern",
        code = "LU026",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoRedundantPattern,
    "Disallow patterns that bind nothing"
}

impl LintRule for NoRedundantPattern {
    fn meta(&self) -> &'static LintMeta {
        NoRedundantPattern::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // check let/const declarations with patterns that bind nothing
        for node_id in ctx.dir.iter_nodes::<dir::Declarator>() {
            let declarator = ctx.dir.get(node_id);

            // only check destructuring patterns
            let pattern = ctx.dir.get(declarator.pattern);
            let is_destructuring = matches!(
                pattern,
                dir::Pattern::Object { .. }
                    | dir::Pattern::Sequence { .. }
                    | dir::Pattern::Tuple { .. }
                    | dir::Pattern::NominalObject { .. }
                    | dir::Pattern::Newtype { .. }
            );

            if !is_destructuring {
                continue;
            }

            // check if the pattern binds anything
            if !binds_anything(ctx, declarator.pattern) {
                let severity = ctx.get_effective_severity(meta, declarator.pattern);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        NO_REDUNDANT_PATTERN.id,
                        NO_REDUNDANT_PATTERN.code,
                        NO_REDUNDANT_PATTERN.category,
                        severity,
                        "pattern binds no values",
                        ctx.dir.get_span(declarator.pattern),
                    )
                    .label("this destructuring doesn't bind any values"),
                );
            }
        }
    }
}

/// Check if a pattern binds any values (not just wildcards).
fn binds_anything(ctx: &LintModuleContext<'_>, pattern_id: dir::LocalNodeId<dir::Pattern>) -> bool {
    let pattern = ctx.dir.get(pattern_id);

    match pattern {
        // wildcard binds nothing
        dir::Pattern::Wildcard => false,

        // assignment patterns bind through the wrapped pattern
        dir::Pattern::Assign { pattern, .. } => binds_anything(ctx, *pattern),

        // binding always binds something
        dir::Pattern::Binding { .. } => true,

        // expression patterns don't bind (they match)
        dir::Pattern::Expression { .. }
        | dir::Pattern::Range { .. }
        | dir::Pattern::TypeExpression { .. } => false,

        // check nested patterns
        dir::Pattern::Object { fields }
        | dir::Pattern::Sequence { fields }
        | dir::Pattern::Tuple { fields } => fields
            .iter()
            .any(|field_id| field_binds_anything(ctx, *field_id)),

        dir::Pattern::NominalObject { fields, .. } | dir::Pattern::Newtype { fields, .. } => fields
            .iter()
            .any(|field_id| field_binds_anything(ctx, *field_id)),

        // union patterns bind if any arm binds
        dir::Pattern::Union { patterns } => patterns.iter().any(|p| binds_anything(ctx, *p)),

        // stack access patterns bind if the inner pattern binds
        dir::Pattern::Must(inner)
        | dir::Pattern::BorrowOf { right: inner, .. }
        | dir::Pattern::MoveOf { right: inner, .. }
        | dir::Pattern::DereferenceOf { right: inner } => binds_anything(ctx, *inner),
    }
}

/// Check if a pattern field binds anything.
fn field_binds_anything(
    ctx: &LintModuleContext<'_>,
    field_id: dir::LocalNodeId<dir::PatternField>,
) -> bool {
    let field = ctx.dir.get(field_id);

    match field {
        dir::PatternField::Named { pattern, .. } => {
            // named field with no pattern binds by name
            if pattern.is_none() {
                return true;
            }

            // if there's a pattern, check if it binds
            pattern.is_some_and(|pattern_id| binds_anything(ctx, pattern_id))
        }

        // computed fields bind if their nested pattern binds
        dir::PatternField::Computed { pattern, .. } => binds_anything(ctx, *pattern),

        // positional field binds if its pattern binds
        dir::PatternField::Positional { pattern, .. } => binds_anything(ctx, *pattern),

        // spread binds if its nested pattern binds
        dir::PatternField::Spread { pattern, .. } => {
            pattern.map(|p| binds_anything(ctx, p)).unwrap_or(false)
        }

        // elision doesn't bind
        dir::PatternField::Elision => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_all_wildcards_object() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_detects_all_wildcards_object.ds",
            r#"
const { a: _, b: _ } = obj
"#,
        );
        test.result(result).assert_lint("no-redundant-pattern");
    }

    #[test]
    fn test_detects_all_wildcards_array() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_detects_all_wildcards_array.ds",
            r#"
const [_, _] = arr
"#,
        );
        test.result(result).assert_lint("no-redundant-pattern");
    }

    #[test]
    fn test_allows_binding_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_allows_binding_pattern.ds",
            r#"
const { a, b } = obj
"#,
        );
        test.result(result).assert_no_lint("no-redundant-pattern");
    }

    #[test]
    fn test_allows_mixed_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_allows_mixed_pattern.ds",
            r#"
const { a, _b } = obj
"#,
        );
        test.result(result).assert_no_lint("no-redundant-pattern");
    }

    #[test]
    fn test_allows_array_with_binding() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_allows_array_with_binding.ds",
            r#"
const [_, x] = arr
"#,
        );
        test.result(result).assert_no_lint("no-redundant-pattern");
    }

    #[test]
    fn test_allows_simple_binding() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantPattern);
        let result = test.lint(
            "no_redundant_pattern/test_allows_simple_binding.ds",
            r#"
const x = getValue()
"#,
        );
        test.result(result).assert_no_lint("no-redundant-pattern");
    }
}
