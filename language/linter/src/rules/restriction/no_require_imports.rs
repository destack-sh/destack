use destack_base::StringId;
use destack_dir::{self as dir, DependencySource};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_global_qualified_member;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

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
        fixable = Sometimes,
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
            let mut diagnostic = LintDiagnostic::new(
                NO_REQUIRE_IMPORTS.id,
                NO_REQUIRE_IMPORTS.code,
                NO_REQUIRE_IMPORTS.category,
                severity,
                "require() import usage",
                ctx.module.file_id,
                span,
            )
            .with_label("use ESM import syntax instead of require()");

            // compute fixes only when requested by the runner
            if ctx.include_fixes
                && let Some(fix) = no_require_import_fix(
                    ctx,
                    expression_id,
                    expression,
                    &global_qualifiers,
                    require_name,
                )
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
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

/// Build a fix for one supported require() import form.
fn no_require_import_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: StringId,
) -> Option<LintFix> {
    // prefer import alias rewrites first
    if let Some(fix) = require_import_alias_fix(ctx, expression) {
        return Some(fix);
    }

    // then handle side effect require() statements
    require_side_effect_fix(
        ctx,
        expression_id,
        expression,
        global_qualifiers,
        require_name,
    )
}

/// Build an unsafe fix for `import A = require("x")` alias declarations.
fn require_import_alias_fix(
    ctx: &LintModuleDirContext<'_>,
    expression: &dir::Expression,
) -> Option<LintFix> {
    let dir::Expression::Declaration { declaration } = expression else {
        return None;
    };

    let declaration_id = *declaration;
    let declaration = ctx.tree.get(declaration_id);
    let dir::Declaration::ImportAlias {
        descriptor,
        kind,
        target: dir::ImportAliasTarget::Require { target },
    } = declaration
    else {
        return None;
    };

    // keep non exported canonical aliases only
    if descriptor.export.is_some() {
        return None;
    }

    let Some(dir::Name::Identifier(name_id)) = descriptor.name else {
        return None;
    };

    let local_name = ctx.program.strings.get(name_id);
    let target_text = escape_import_target(ctx.program.strings.get(*target).as_ref());
    let prefix = if *kind == dir::DependencyKind::Type {
        "import type"
    } else {
        "import"
    };
    let replacement = format!("{prefix} {} from \"{target_text}\";", local_name.as_ref());

    let span = require_import_alias_span(ctx, declaration_id)?;
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::r#unsafe("Rewrite require alias to ESM import").with_edits(edits))
}

/// Resolve one source span for the import-alias statement prefix.
fn require_import_alias_span(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> Option<Span> {
    let declaration_span = ctx.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);

    let statement_length = declaration_text
        .find(';')
        .map(|offset| offset + 1)
        .or_else(|| declaration_text.find('\n'))
        .unwrap_or(declaration_text.len());

    if statement_length == 0 {
        return None;
    }

    Some(Span::new(
        declaration_span.file,
        declaration_span.start,
        declaration_span.start + statement_length as u32,
    ))
}

/// Build an unsafe fix for side effect `require("x")` statements.
fn require_side_effect_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: StringId,
) -> Option<LintFix> {
    let dir::Expression::Call {
        static_arguments,
        dynamic_arguments,
        ..
    } = expression
    else {
        return None;
    };

    // keep call forms without static arguments
    if static_arguments
        .as_ref()
        .is_some_and(|args| !args.is_empty())
    {
        return None;
    }

    // keep only require-like callees
    if !expression_uses_require_import(ctx.tree, expression, global_qualifiers, require_name) {
        return None;
    }

    // keep one positional string argument
    if dynamic_arguments.len() != 1 {
        return None;
    }

    let argument = ctx.tree.get(dynamic_arguments[0]);
    let dir::Argument::Positional { value, .. } = argument else {
        return None;
    };
    let argument_value = ctx.tree.get(*value);
    let dir::Expression::ScalarLiteral {
        value: dir::ScalarLiteral::String(target),
    } = argument_value
    else {
        return None;
    };

    // keep standalone statement calls only
    let parent = ctx.tree.get_parent(expression_id.id)?;
    if parent.ty != dir::NodeType::Expression {
        return None;
    }

    let statement_id = parent.into_typed::<dir::Expression>();
    let statement = ctx.tree.get(statement_id);
    let dir::Expression::Statement { statement } = statement else {
        return None;
    };
    if *statement != expression_id {
        return None;
    }

    let target_text = escape_import_target(ctx.program.strings.get(*target).as_ref());
    let replacement = format!("import \"{target_text}\";");
    let statement_span = ctx.get_span(statement_id);
    let edits = ctx
        .edit_builder()
        .replace(statement_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Rewrite require() side effect import to ESM import").with_edits(edits))
}

/// Escape one module import target for a double quoted import string.
fn escape_import_target(target: &str) -> String {
    target.replace('\\', "\\\\").replace('"', "\\\"")
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

    /// Unsafely rewrite one import-equals require alias to ESM.
    #[test]
    fn test_fix_rewrites_import_equals_require_alias() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_fix_rewrites_import_equals_require_alias.ds",
            r#"
import fs = require("fs");
"#,
        );
        test.result(result)
            .assert_lint("no-require-imports")
            .assert_unsafe_fixed(
                r#"
import fs from "fs";
"#,
            );
    }

    /// Keep trailing statements when rewriting import-equals aliases.
    #[test]
    fn test_mutation_fix_rewrites_import_equals_require_alias_with_following_statement() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_mutation_fix_rewrites_import_equals_require_alias_with_following_statement.ds",
            r#"
import fs = require("fs");
fs;
"#,
        );
        test.result(result)
            .assert_lint("no-require-imports")
            .assert_has_fix("no-require-imports");
    }

    /// Unsafely rewrite side effect require call statements.
    #[test]
    fn test_fix_rewrites_side_effect_require_statement() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_fix_rewrites_side_effect_require_statement.ds",
            r#"
require("fs");
"#,
        );
        test.result(result)
            .assert_lint("no-require-imports")
            .assert_unsafe_fixed(
                r#"
import "fs";
"#,
            );
    }

    /// Keep exported import alias forms without auto-fix.
    #[test]
    fn test_no_fix_for_export_import_equals_require_alias() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_no_fix_for_export_import_equals_require_alias.ds",
            r#"
export import fs = require("fs");
fs;
"#,
        );
        test.result(result)
            .assert_lint("no-require-imports")
            .assert_has_no_fix("no-require-imports");
    }

    /// Keep require calls used as value expressions without auto-fix.
    #[test]
    fn test_no_fix_for_require_call_used_as_value() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_no_fix_for_require_call_used_as_value.ds",
            r#"
const fs = require("fs");
fs;
"#,
        );
        test.result(result)
            .assert_lint("no-require-imports")
            .assert_has_no_fix("no-require-imports");
    }
}
