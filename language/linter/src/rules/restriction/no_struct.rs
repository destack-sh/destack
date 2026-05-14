use destack_dir::{self as dir, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow struct declarations.
    ///
    /// Some codebases prefer classes over structs for consistency with
    /// existing TypeScript patterns or when identity semantics are needed.
    /// This rule enforces class only object definitions.
    #[lint(
        id = "no-struct",
        code = "LR027",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoStruct,
    "Disallow struct declarations"
}

impl LintRule for NoStruct {
    fn meta(&self) -> &'static LintMeta {
        NoStruct::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate declarations
        for node_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(node_id);
            if !matches!(declaration, Declaration::Struct(_)) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.dir.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_STRUCT.id,
                    NO_STRUCT.code,
                    NO_STRUCT.category,
                    severity,
                    "struct declaration is not allowed",
                    span,
                )
                .label("use class instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_struct() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_detects_struct.ds",
            r#"
struct Point {
    x: int32;
    y: int32;
}
"#,
        );
        test.result(result).assert_lint("no-struct");
    }

    #[test]
    fn test_detects_exported_struct() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_detects_exported_struct.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;
}
"#,
        );
        test.result(result).assert_lint("no-struct");
    }

    #[test]
    fn test_allows_class() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_allows_class.ts",
            r#"
class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_allows_interface() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_allows_interface.ts",
            r#"
interface MyInterface {
    foo(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_allows_type() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_allows_type.ts",
            r#"
type Point = { x: number, y: number };
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoStruct);
        let result = test.lint(
            "no_struct/test_skips_declaration_file_by_default.d.ds",
            r#"
struct Point {
    x: int32;
    y: int32;
}
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoStruct).with_options(|options| {
            options.include_declaration_files = true;
        });
        let result = test.lint(
            "no_struct/test_includes_declaration_file_when_enabled.d.ds",
            r#"
struct Point {
    x: int32;
    y: int32;
}
"#,
        );
        test.result(result).assert_lint("no-struct");
    }
}
