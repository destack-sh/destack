use std::sync::Arc;

use regex::Regex;

use destack_core::StringId;
use destack_dir::{self as dir, ImportSource, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    compiled_allowed_require_import_patterns, expression_is_global_qualified_member,
    expression_is_standalone_statement, expression_static_string_literal,
    expression_unwrap_transparent, statement_prefix_span,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow CommonJS style `require()` imports.
    ///
    /// Use ESM imports for static analysis, tree shaking, and consistency.
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
        let mut visitor = NoRequireImportsVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags require import usage.
struct NoRequireImportsVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The `require` name id.
    require_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// Static require target allow patterns.
    allowed_target_patterns: Arc<[Regex]>,
    /// Whether import-equals require aliases are allowed.
    allow_require_import_aliases: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoRequireImportsVisitor<'a, 'b> {
    /// Build a visitor for require import checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let require_name = ctx.string_id("require");
        let global_qualifiers = ctx.global_qualifier_symbols();
        let allowed_target_patterns = compiled_allowed_require_import_patterns(
            &ctx.options.restriction.allowed_require_import_patterns,
        );
        let allow_require_import_aliases = ctx.options.restriction.allow_require_import_aliases;

        Self {
            ctx,
            meta,
            require_name,
            global_qualifiers,
            allowed_target_patterns,
            allow_require_import_aliases,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one expression for require import usage.
    fn check_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if self.allow_require_import_aliases
            && expression_is_require_import_alias(self.ctx.tree, expression)
        {
            return;
        }

        if let Some(target) = require_import_target(
            self.ctx,
            expression,
            &self.global_qualifiers,
            self.require_name,
        ) && require_target_is_allowed(
            self.ctx.strings.get(target).as_ref(),
            &self.allowed_target_patterns,
        ) {
            return;
        }

        if !expression_uses_require_import(
            self.ctx,
            expression,
            &self.global_qualifiers,
            self.require_name,
        ) {
            return;
        }

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // resolve diagnostic span
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_REQUIRE_IMPORTS.id,
            NO_REQUIRE_IMPORTS.code,
            NO_REQUIRE_IMPORTS.category,
            severity,
            "require() import usage",
            span,
        )
        .label("use an ESM import instead of require()");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = no_require_import_fix(
                self.ctx,
                expression_id,
                expression,
                &self.global_qualifiers,
                self.require_name,
            )
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for NoRequireImportsVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.check_expression(id, expression);
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when an expression is `import foo = require("foo")`.
fn expression_is_require_import_alias(tree: &dir::Tree, expression: &dir::Expression) -> bool {
    let dir::Expression::Declaration(declaration) = expression else {
        return false;
    };

    let declaration = tree.get(*declaration);
    matches!(
        declaration,
        dir::Declaration::ImportAlias(dir::ImportAliasDeclaration {
            target: dir::ImportAliasTarget::Require { .. },
            ..
        })
    )
}

/// Return true when an expression represents a require() import.
fn expression_uses_require_import(
    ctx: &LintModuleDirContext<'_>,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: StringId,
) -> bool {
    // check require based dependency expressions first
    match expression {
        dir::Expression::Declaration(declaration) => {
            let declaration = ctx.tree.get(*declaration);
            if let dir::Declaration::ImportAlias(dir::ImportAliasDeclaration {
                target: dir::ImportAliasTarget::Require { .. },
                ..
            }) = declaration
            {
                return true;
            }
        }
        dir::Expression::Import { source, .. } => {
            if *source == ImportSource::RequireCall || *source == ImportSource::ImportEquals {
                return true;
            }
        }
        _ => {}
    }

    // inspect direct require calls
    let dir::Expression::Call { left, .. } = expression else {
        return false;
    };
    let callee_id = expression_unwrap_transparent(ctx.tree, *left);
    let callee = ctx.tree.get(callee_id);

    // match bare require paths
    if let dir::Expression::Path { path, .. } = callee
        && path.segments.len() == 1
        && path.segments[0] == require_name
    {
        return true;
    }

    expression_is_global_qualified_member(ctx, callee_id, global_qualifiers, require_name)
}

/// Return the static target string for one require-based import form.
fn require_import_target(
    ctx: &LintModuleDirContext<'_>,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: StringId,
) -> Option<StringId> {
    match expression {
        dir::Expression::Declaration(declaration) => {
            let declaration = ctx.tree.get(*declaration);
            let dir::Declaration::ImportAlias(dir::ImportAliasDeclaration {
                target: dir::ImportAliasTarget::Require { target },
                ..
            }) = declaration
            else {
                return None;
            };

            return Some(*target);
        }
        dir::Expression::Import { source, target, .. }
            if *source == ImportSource::RequireCall || *source == ImportSource::ImportEquals =>
        {
            let dir::ImportTarget::String(target) = target else {
                return None;
            };

            return Some(*target);
        }
        _ => {}
    }

    let dir::Expression::Call { arguments, .. } = expression else {
        return None;
    };
    if arguments.len() != 1 {
        return None;
    }

    if !expression_uses_require_import(ctx, expression, global_qualifiers, require_name) {
        return None;
    }

    let argument = ctx.tree.get(arguments[0]);
    let dir::Argument::Positional { value, .. } = argument else {
        return None;
    };

    expression_static_string_literal(ctx.tree, *value)
}

/// Return true when one static require target matches an allowed pattern.
fn require_target_is_allowed(target: &str, allowed_target_patterns: &[Regex]) -> bool {
    allowed_target_patterns
        .iter()
        .any(|pattern| pattern.is_match(target))
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
    let dir::Expression::Declaration(declaration) = expression else {
        return None;
    };

    // resolve declaration id
    let declaration_id = *declaration;
    let declaration = ctx.tree.get(declaration_id);
    let dir::Declaration::ImportAlias(declaration) = declaration else {
        return None;
    };

    // keep non exported aliases only
    if declaration.export.is_some() {
        return None;
    }

    // require optional structure
    let dir::Name::Identifier(name_id) = declaration.name else {
        return None;
    };

    // render the esm import replacement for this alias
    let local_name = ctx.strings.get(name_id);
    let dir::ImportAliasTarget::Require { target } = declaration.target else {
        return None;
    };
    let target_text = escape_import_target(ctx.strings.get(target).as_ref());
    let prefix = if declaration.space == dir::DependencySpace::Type {
        "import type"
    } else {
        "import"
    };
    let replacement = format!("{prefix} {} from \"{target_text}\";", local_name.as_ref());

    // replace the original declaration statement prefix
    let declaration_span = ctx.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);
    let span = statement_prefix_span(declaration_span, declaration_text)?;
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::r#unsafe("Rewrite require alias to ESM import").with_edits(edits))
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
        generic_arguments,
        arguments,
        ..
    } = expression
    else {
        return None;
    };

    // keep call forms without static arguments
    if !generic_arguments.is_empty() {
        return None;
    }

    // keep only require like callees
    if !expression_uses_require_import(ctx, expression, global_qualifiers, require_name) {
        return None;
    }

    // keep one positional string argument
    if arguments.len() != 1 {
        return None;
    }

    // require one positional call argument
    let argument = ctx.tree.get(arguments[0]);
    let dir::Argument::Positional { value, .. } = argument else {
        return None;
    };
    let target = expression_static_string_literal(ctx.tree, *value)?;

    // keep standalone statement calls only
    if !expression_is_standalone_statement(ctx.tree, expression_id) {
        return None;
    }

    // render one side effect esm import replacement
    let target_text = escape_import_target(ctx.strings.get(target).as_ref());
    let replacement = format!("import \"{target_text}\";");
    let statement_span = ctx.get_span(expression_id);
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

    /// Report computed global require() import calls.
    #[test]
    fn test_flags_global_computed_require_call() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_flags_global_computed_require_call.ds",
            r#"
const fs = globalThis["require"]("fs");
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Allow ESM imports.
    #[test]
    fn test_allows_import_declaration() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_allows_import_declaration.ds",
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

    /// Allow import-equals require declarations when configured.
    #[test]
    fn test_allows_import_equals_require_when_enabled() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports).with_options(|options| {
            options.restriction.allow_require_import_aliases = true;
        });
        let result = test.lint_dir(
            "no_require_imports/test_allows_import_equals_require_when_enabled.ds",
            r#"
import fs = require("fs");
fs;
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
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

    /// Allow require calls that match configured target patterns.
    #[test]
    fn test_allows_require_call_with_allowed_target_pattern() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports).with_options(|options| {
            options.restriction.allowed_require_import_patterns =
                vec!["/package\\.json$".to_string()];
        });
        let result = test.lint_dir(
            "no_require_imports/test_allows_require_call_with_allowed_target_pattern.ds",
            r#"
const pkg = require("../package.json");
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Allow template-literal require calls that match configured target patterns.
    #[test]
    fn test_allows_template_require_call_with_allowed_target_pattern() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports).with_options(|options| {
            options.restriction.allowed_require_import_patterns =
                vec!["/package\\.json$".to_string()];
        });
        let result = test.lint_dir(
            "no_require_imports/test_allows_template_require_call_with_allowed_target_pattern.ds",
            r#"
const pkg = require(`../package.json`);
"#,
        );
        test.result(result).assert_no_lint("no-require-imports");
    }

    /// Keep reporting require calls when the configured target patterns do not match.
    #[test]
    fn test_flags_require_call_when_allowed_target_pattern_does_not_match() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports).with_options(|options| {
            options.restriction.allowed_require_import_patterns =
                vec!["/package\\.json$".to_string()];
        });
        let result = test.lint_dir(
            "no_require_imports/test_flags_require_call_when_allowed_target_pattern_does_not_match.ds",
            r#"
const fs = require("fs");
"#,
        );
        test.result(result).assert_lint("no-require-imports");
    }

    /// Unsafely rewrite one import equals require alias to ESM.
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

    /// Keep trailing statements when rewriting import equals aliases.
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

    /// Unsafely rewrite side effect require template statement.
    #[test]
    fn test_fix_rewrites_static_template_require_statement() {
        let test = TestProgram::for_rule_with_prelude(NoRequireImports);
        let result = test.lint_dir(
            "no_require_imports/test_fix_rewrites_static_template_require_statement.ds",
            r#"
require(`fs`);
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
