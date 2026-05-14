use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow continue statements.
    ///
    /// The `continue` statement can make code harder to follow. Consider
    /// restructuring the loop logic or using early returns in helper functions.
    #[lint(
        id = "no-continue",
        code = "LR008",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoContinue,
    "Disallow continue statements"
}

impl LintRule for NoContinue {
    fn meta(&self) -> &'static LintMeta {
        NoContinue::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            if !matches!(expression, dir::Expression::Continue { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.dir.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_CONTINUE.id,
                    NO_CONTINUE.code,
                    NO_CONTINUE.category,
                    severity,
                    "unexpected use of continue statement",
                    span,
                )
                .label("avoid continue statements"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_continue() {
        let test = TestProgram::for_rule_without_prelude(NoContinue);
        let result = test.lint(
            "no_continue/test_detects_continue.ts",
            r#"
for (let i = 0; i < 10; i++) {
    if (i === 5) continue;
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("no-continue");
    }

    #[test]
    fn test_detects_labeled_continue() {
        let test = TestProgram::for_rule_without_prelude(NoContinue);
        let result = test.lint(
            "no_continue/test_detects_labeled_continue.ts",
            r#"
outer: for (let i = 0; i < 10; i++) {
    for (let j = 0; j < 10; j++) {
        continue outer;
    }
}
"#,
        );
        test.result(result).assert_lint("no-continue");
    }

    #[test]
    fn test_allows_break() {
        let test = TestProgram::for_rule_without_prelude(NoContinue);
        let result = test.lint(
            "no_continue/test_allows_break.ts",
            r#"
for (let i = 0; i < 10; i++) {
    if (i === 5) break;
}
"#,
        );
        test.result(result).assert_no_lint("no-continue");
    }

    #[test]
    fn test_detects_continue_in_do_while() {
        let test = TestProgram::for_rule_without_prelude(NoContinue);
        let result = test.lint(
            "no_continue/test_detects_continue_in_do_while.ts",
            r#"
let i = 0;
do {
    i += 1;
    continue;
} while (i < 3);
"#,
        );
        test.result(result).assert_lint("no-continue");
    }

    #[test]
    fn test_detects_continue_in_while() {
        let test = TestProgram::for_rule_without_prelude(NoContinue);
        let result = test.lint(
            "no_continue/test_detects_continue_in_while.ts",
            r#"
let i = 0;
while (i < 3) {
    i += 1;
    continue;
}
"#,
        );
        test.result(result).assert_lint("no-continue");
    }
}
