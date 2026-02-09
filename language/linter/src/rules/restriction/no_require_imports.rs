use destack_base::StringId;
use destack_dir::{self as dir, DependencySource};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_global_qualified_member;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow CommonJS style `require()` imports.
    ///
    /// Use ESM import syntax for static analysis, tree shaking, and consistency.
    #[lint(
        id = "no-require-imports",
        code = "LR033",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoRequireImports,
    "Disallow require() imports"
}

impl LintRule for NoRequireImports {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoRequireImports::meta()
    }

    /// Check module DIR nodes for require() import calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let require_name = ctx.program.strings.intern("require");
        let global_qualifiers = ctx.global_qualifier_symbols();

        // check module expressions for CommonJS require imports
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            if !expression_uses_require_import(
                ctx.tree,
                expression,
                &global_qualifiers,
                require_name,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(expression_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_REQUIRE_IMPORTS.id,
                    NO_REQUIRE_IMPORTS.code,
                    NO_REQUIRE_IMPORTS.category,
                    severity,
                    "require() import usage",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use ESM import syntax instead of require()"),
            );
        }
    }
}

/// Return true when an expression represents a require() import.
fn expression_uses_require_import(
    tree: &dir::NodeTree,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: StringId,
) -> bool {
    // check require-based dependency expressions first
    match expression {
        dir::Expression::Declaration { declaration } => {
            let declaration = tree.get(*declaration);
            if let dir::Declaration::ImportAlias {
                target: dir::ImportAliasTarget::Require { .. },
                ..
            } = declaration
            {
                return true;
            }
        }
        dir::Expression::Import { source, .. }
        | dir::Expression::UnresolvedImport { source, .. } => {
            if *source == DependencySource::RequireCall || *source == DependencySource::ImportEquals
            {
                return true;
            }
        }
        _ => {}
    }

    // fall back to direct call detection for unresolved or non lowered forms
    let dir::Expression::Call { left, .. } = expression else {
        return false;
    };
    let callee = tree.get(*left);
    if let dir::Expression::UnresolvedPath { path, .. } = callee
        && path.segments.len() == 1
        && path.segments[0] == require_name
    {
        return true;
    }

    expression_is_global_qualified_member(tree, *left, global_qualifiers, require_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report require() import calls.
    #[test]
    fn test_flags_require_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_require_call.ds",
            r#"
const fs = require("fs");
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Report global require() import calls.
    #[test]
    fn test_flags_global_require_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_global_require_call.ds",
            r#"
const fs = globalThis.require("fs");
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Allow ESM imports.
    #[test]
    fn test_allows_import_syntax() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_import_syntax.ds",
            r#"
import { readFile } from "fs";
readFile;
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Allow local shadowed require bindings.
    #[test]
    fn test_allows_shadowed_require_symbol() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_shadowed_require_symbol.ds",
            r#"
function require(name: string): unknown {
    return name;
}

const fs = require("fs");
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Allow non import member calls on require.
    #[test]
    fn test_allows_require_member_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_require_member_call.ds",
            r#"
const name = require.resolve("fs");
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Report import equals require declarations.
    #[test]
    fn test_flags_import_equals_require_declaration() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_import_equals_require_declaration.ds",
            r#"
import fs = require("fs");
fs;
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Report exported import equals require declarations.
    #[test]
    fn test_flags_export_import_equals_require_declaration() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_export_import_equals_require_declaration.ds",
            r#"
export import fs = require("fs");
fs;
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Report import type equals require declarations.
    #[test]
    fn test_flags_import_type_equals_require_declaration() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_import_type_equals_require_declaration.ds",
            r#"
import type React = require("react");
React;
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Allow locally shadowed globalThis.require calls.
    #[test]
    fn test_allows_shadowed_global_require_member_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_shadowed_global_require_member_call.ds",
            r#"
let globalThis = {
    require(name: string): string {
        return name;
    },
};

const fs = globalThis.require("fs");
fs;
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Allow non require dynamic imports.
    #[test]
    fn test_allows_dynamic_import_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_dynamic_import_call.ds",
            r#"
const fs = import("fs");
fs;
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }
}
