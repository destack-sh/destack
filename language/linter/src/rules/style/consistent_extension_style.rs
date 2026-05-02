use crate::LintMeta;
use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent extension naming style.
    ///
    /// Extensions can be named (`extension Foo of Bar`) or anonymous (`extension of Bar`).
    /// This rule prefers named extensions for better discoverability and debugging.
    #[lint(
        id = "consistent-extension-style",
        code = "LY005",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentExtensionStyle,
    "Enforce consistent extension naming"
}

impl LintRule for ConsistentExtensionStyle {
    fn meta(&self) -> &'static LintMeta {
        ConsistentExtensionStyle::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            // look for extension declarations
            let Declaration::Extension(declaration) = decl else {
                continue;
            };

            // check if extension is anonymous (no name)
            if declaration.name.is_none() {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        CONSISTENT_EXTENSION_STYLE.id,
                        CONSISTENT_EXTENSION_STYLE.code,
                        CONSISTENT_EXTENSION_STYLE.category,
                        severity,
                        "extension should have a name",
                        ctx.tree.get_span(node_id),
                    )
                    .label("add a name like `extension MyExt of ...`"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_named_extension() {
        let test = TestProgram::for_rule_without_prelude(ConsistentExtensionStyle);
        let result = test.lint_ast(
            "consistent_extension_style/test_allows_named_extension.ds",
            r#"
extension StringUtils of string {
    function capitalize(): string {
        return this
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-extension-style");
    }

    #[test]
    fn test_detects_anonymous_extension() {
        let test = TestProgram::for_rule_without_prelude(ConsistentExtensionStyle);
        let result = test.lint_ast(
            "consistent_extension_style/test_detects_anonymous_extension.ds",
            r#"
extension of string {
    function capitalize(): string {
        return this
    }
}
"#,
        );
        test.result(result)
            .assert_lint("consistent-extension-style");
    }

    #[test]
    fn test_allows_named_generic_extension() {
        let test = TestProgram::for_rule_without_prelude(ConsistentExtensionStyle);
        let result = test.lint_ast(
            "consistent_extension_style/test_allows_named_generic_extension.ds",
            r#"
extension ArrayUtils<T> of Array<T> {
    function first(): T | undefined {
        return this[0]
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-extension-style");
    }
}
