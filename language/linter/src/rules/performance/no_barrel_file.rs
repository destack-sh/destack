use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow barrel files that re-export everything.
    ///
    /// Barrel files (like `index.ts` files that only re-export from other modules)
    /// can hurt tree-shaking and bundle size because bundlers may have difficulty
    /// determining which exports are actually used.
    ///
    /// Bad:
    /// ```
    /// export * from "./foo";
    /// export * from "./bar";
    /// export { baz } from "./baz";
    /// ```
    ///
    /// Good: Import directly from the source modules.
    #[lint(
        id = "no-barrel-file",
        code = "LP002",
        category = Performance,
        level = Ast
    )]
    pub NoBarrelFile,
    "Disallow barrel files"
}

impl LintRule for NoBarrelFile {
    fn meta(&self) -> &'static crate::LintMeta {
        NoBarrelFile::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let mut export_count = 0;
        let mut reexport_count = 0;
        let mut has_other_code = false;
        let mut first_export_id: Option<ast::LocalNodeId<ast::Expression>> = None;

        // count exports and check for other code
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            match expression {
                Expression::Export { target, .. } => {
                    export_count += 1;
                    if first_export_id.is_none() {
                        first_export_id = Some(node_id);
                    }
                    // re-export if it has a target (from clause)
                    if target.is_some() {
                        reexport_count += 1;
                    }
                }
                Expression::Import { .. } => {
                    // imports are fine in barrel files
                }
                Expression::Declaration(_) => {
                    // declarations indicate real code
                    has_other_code = true;
                }
                Expression::Let { .. } => {
                    // let bindings indicate real code
                    has_other_code = true;
                }
                _ => {
                    // other top-level expressions are rare but indicate non-barrel (?)
                }
            }
        }

        // a barrel file has:
        // 1. at least 2 exports (a single re-export is fine)
        // 2. all exports are re-exports (have a target/from clause)
        // 3. no other code besides imports
        if export_count >= 2
            && export_count == reexport_count
            && !has_other_code
            && let Some(node_id) = first_export_id
        {
            ctx.report(
                LintDiagnostic::new(
                    NO_BARREL_FILE.id,
                    NO_BARREL_FILE.code,
                    NO_BARREL_FILE.category,
                    severity,
                    format!("barrel file with {export_count} re-exports hurts tree-shaking"),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("import directly from source modules instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_barrel_file_with_star_exports() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
export * from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_detects_barrel_file_with_named_exports() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export { foo } from "./foo"
export { bar, baz } from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_detects_mixed_barrel_file() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
export { bar } from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_single_reexport() {
        // a single re-export is fine, not considered a barrel file
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_module_with_real_code() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export * from "./utils"

export function main() {
    console.log("hello")
}
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_local_exports() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export function foo() {}
export const bar = 42
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_mixed_local_and_reexport() {
        let test = TestProgram::for_rule(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export { foo } from "./foo"
export function bar() {}
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }
}
