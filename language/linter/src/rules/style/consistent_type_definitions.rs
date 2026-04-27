use crate::LintMeta;
use destack_ast::{self as ast, Declaration, TypeExpression};
use destack_workspace::{LintSeverity, TypeDefinitionStyle};

use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type definition style.
    ///
    /// Choose between `type` aliases and `interface` declarations.
    /// Configure via `type_definition_style` option.
    #[lint(
        id = "consistent-type-definitions",
        code = "LY006",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentTypeDefinitions,
    "Enforce consistent type definition style"
}

impl LintRule for ConsistentTypeDefinitions {
    fn meta(&self) -> &'static LintMeta {
        ConsistentTypeDefinitions::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let preferred_style = ctx.options.style.type_definition_style;

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            match (preferred_style, decl) {
                // prefer type, found interface (structural only, not newtype interface)
                (TypeDefinitionStyle::Type, Declaration::Interface(declaration))
                    if !declaration.is_nominal =>
                {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let mut diagnostic = LintDiagnostic::new(
                        CONSISTENT_TYPE_DEFINITIONS.id,
                        CONSISTENT_TYPE_DEFINITIONS.code,
                        CONSISTENT_TYPE_DEFINITIONS.category,
                        severity,
                        "use `type` instead of `interface`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("prefer type alias");
                    if ctx.compute_fixes
                        && let Some(fix) = interface_to_type_fix(ctx, node_id, declaration)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
                // prefer interface, found type alias with object value (structural only)
                (TypeDefinitionStyle::Interface, Declaration::Type(declaration))
                    if !declaration.is_nominal =>
                {
                    // only flag if the value is an object type expression
                    let value_expr = ctx.tree.get(declaration.value);
                    if is_object_type_expression(value_expr) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        let mut diagnostic = LintDiagnostic::new(
                            CONSISTENT_TYPE_DEFINITIONS.id,
                            CONSISTENT_TYPE_DEFINITIONS.code,
                            CONSISTENT_TYPE_DEFINITIONS.category,
                            severity,
                            "use `interface` instead of `type`",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("prefer interface declaration");
                        if ctx.compute_fixes
                            && let Some(fix) = type_to_interface_fix(ctx, node_id, declaration)
                        {
                            diagnostic = diagnostic.with_fix(fix);
                        }

                        ctx.report(diagnostic);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check if an expression represents an object type.
fn is_object_type_expression(expr: &TypeExpression) -> bool {
    matches!(expr, TypeExpression::Object { .. })
}

/// Build an unsafe interface to type alias rewrite.
fn interface_to_type_fix(
    ctx: &LintAstContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
    declaration: &ast::InterfaceDeclaration,
) -> Option<LintFix> {
    // keep plain structural interfaces only
    let name = declaration.name?;
    if !declaration.generic_parameters.is_empty() || !declaration.extends.is_empty() {
        return None;
    }

    let declaration_span = ctx.tree.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span).to_string();
    let interface_index = declaration_text.find("interface")?;
    let open_brace_index = declaration_text.find('{')?;
    let close_brace_index = declaration_text.rfind('}')?;
    if open_brace_index >= close_brace_index {
        return None;
    }

    let name_text = ctx.strings.get(name.string());
    let prefix = &declaration_text[..interface_index];
    let body = &declaration_text[open_brace_index..=close_brace_index];
    let replacement = format!("{prefix}type {} = {body}", name_text.as_ref());

    let edits = ctx
        .edit_builder()
        .replace(declaration_span, replacement)
        .into_edits();
    Some(
        LintFix::r#unsafe("Rewrite interface declaration into equivalent type alias")
            .with_edits(edits),
    )
}

/// Build an unsafe type alias to interface rewrite.
fn type_to_interface_fix(
    ctx: &LintAstContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
    declaration: &ast::TypeDeclaration,
) -> Option<LintFix> {
    // keep plain object type aliases only
    if !declaration.generic_parameters.is_empty() || declaration.mutability.is_some() {
        return None;
    }

    let name = declaration.name;
    let value_expression = ctx.tree.get(declaration.value);
    if !matches!(value_expression, TypeExpression::Object { .. }) {
        return None;
    }

    let declaration_span = ctx.tree.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span).to_string();
    let type_index = declaration_text.find("type")?;
    let value_text = ctx
        .get_span_text(ctx.tree.get_span(declaration.value))
        .to_string();
    if !value_text.trim_start().starts_with('{') {
        return None;
    }

    let name_text = ctx.strings.get(name.string());
    let prefix = &declaration_text[..type_index];
    let replacement = format!("{prefix}interface {} {value_text}", name_text.as_ref());

    let edits = ctx
        .edit_builder()
        .replace(declaration_span, replacement)
        .into_edits();
    Some(
        LintFix::r#unsafe("Rewrite object type alias into equivalent interface declaration")
            .with_edits(edits),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_type_when_type_preferred() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions);
        let result = test.lint_ast(
            "consistent_type_definitions/test_allows_type_when_type_preferred.ds",
            r#"
type Point = { x: int32, y: int32 }
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }

    #[test]
    fn test_detects_interface_when_type_preferred() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions);
        let result = test.lint_ast(
            "consistent_type_definitions/test_detects_interface_when_type_preferred.ds",
            r#"
interface Point {
    x: int32
    y: int32
}
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-definitions")
            .assert_unsafe_fixed(
                r#"
type Point = {
    x: int32,
    y: int32,
};
"#,
            );
    }

    #[test]
    fn test_allows_newtype_interface() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions);
        // newtype interfaces are not flagged (they have different semantics)
        let result = test.lint_ast(
            "consistent_type_definitions/test_allows_newtype_interface.ds",
            r#"
newtype interface Serializable {
    serialize(): string
}
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }

    #[test]
    fn test_allows_type_alias_non_object() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions);
        // type aliases to non-object types are not flagged
        let result = test.lint_ast(
            "consistent_type_definitions/test_allows_type_alias_non_object.ds",
            r#"
type ID = string
type Handler = (event: Event) => void
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }

    #[test]
    fn test_detects_type_alias_when_interface_preferred() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions).with_options(
            |options| options.style.type_definition_style = TypeDefinitionStyle::Interface,
        );
        let result = test.lint_ast(
            "consistent_type_definitions/test_detects_type_alias_when_interface_preferred.ds",
            r#"
type Point = { x: int32, y: int32 }
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-definitions")
            .assert_unsafe_fixed(
                r#"
interface Point {
    x: int32;
    y: int32;
}
"#,
            );
    }

    #[test]
    fn test_no_fix_for_interface_with_heritage() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions);
        let result = test.lint_ast(
            "consistent_type_definitions/test_no_fix_for_interface_with_heritage.ds",
            r#"
interface Point extends Shape {
    x: int32
}
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-definitions")
            .assert_has_no_fix("consistent-type-definitions");
    }

    #[test]
    fn test_no_fix_for_type_alias_with_static_parameters() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions).with_options(
            |options| options.style.type_definition_style = TypeDefinitionStyle::Interface,
        );
        let result = test.lint_ast(
            "consistent_type_definitions/test_no_fix_for_type_alias_with_static_parameters.ds",
            r#"
type Box<T> = { value: T }
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-definitions")
            .assert_has_no_fix("consistent-type-definitions");
    }

    #[test]
    fn test_tuple_alias_not_reported_when_interface_preferred() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeDefinitions).with_options(
            |options| options.style.type_definition_style = TypeDefinitionStyle::Interface,
        );
        let result = test.lint_ast(
            "consistent_type_definitions/test_tuple_alias_not_reported_when_interface_preferred.ds",
            r#"
type Pair = (int32, int32)
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }
}
