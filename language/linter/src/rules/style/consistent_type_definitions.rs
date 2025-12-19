use destack_ast::{self as ast, Declaration, TypeKind};
use destack_workspace::{LintSeverity, TypeDefinitionStyle};

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type definition style.
    ///
    /// Choose between `type` aliases and `interface` declarations.
    /// Configure via `type_definition_style` option.
    #[lint(
        id = "consistent-type-definitions",
        code = "LY023",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentTypeDefinitions,
    "Enforce consistent type definition style"
}

impl LintRule for ConsistentTypeDefinitions {
    fn meta(&self) -> &'static crate::LintMeta {
        ConsistentTypeDefinitions::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let preferred_style = ctx.options.type_definition_style;

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            match (preferred_style, decl) {
                // prefer type, found interface (structural only, not newtype interface)
                (
                    TypeDefinitionStyle::Type,
                    Declaration::Interface {
                        kind: TypeKind::Structural,
                        ..
                    },
                ) => {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    ctx.report(
                        LintDiagnostic::new(
                            CONSISTENT_TYPE_DEFINITIONS.id,
                            CONSISTENT_TYPE_DEFINITIONS.code,
                            CONSISTENT_TYPE_DEFINITIONS.category,
                            severity,
                            "use `type` instead of `interface`",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("prefer type alias"),
                    );
                }
                // prefer interface, found type alias with object value (structural only)
                (
                    TypeDefinitionStyle::Interface,
                    Declaration::Type {
                        kind: TypeKind::Structural,
                        value,
                        ..
                    },
                ) => {
                    // only flag if the value is an object type expression
                    let value_expr = ctx.tree.get(*value);
                    if is_object_type_expression(value_expr) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        ctx.report(
                            LintDiagnostic::new(
                                CONSISTENT_TYPE_DEFINITIONS.id,
                                CONSISTENT_TYPE_DEFINITIONS.code,
                                CONSISTENT_TYPE_DEFINITIONS.category,
                                severity,
                                "use `interface` instead of `type`",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("prefer interface declaration"),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check if an expression represents an object type.
fn is_object_type_expression(expr: &ast::Expression) -> bool {
    matches!(
        expr,
        ast::Expression::ObjectExpression { .. } | ast::Expression::TupleExpression { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_type_when_type_preferred() {
        let test = TestProgram::for_rule(ConsistentTypeDefinitions);
        let result = test.lint_ast(
            "test.ds",
            r#"
type Point = { x: int32, y: int32 }
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }

    #[test]
    fn test_detects_interface_when_type_preferred() {
        let test = TestProgram::for_rule(ConsistentTypeDefinitions);
        let result = test.lint_ast(
            "test.ds",
            r#"
interface Point {
    x: int32
    y: int32
}
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-definitions");
    }

    #[test]
    fn test_allows_newtype_interface() {
        let test = TestProgram::for_rule(ConsistentTypeDefinitions);
        // newtype interfaces are not flagged (they have different semantics)
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ConsistentTypeDefinitions);
        // type aliases to non-object types are not flagged
        let result = test.lint_ast(
            "test.ds",
            r#"
type ID = string
type Handler = (event: Event) => void
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-definitions");
    }
}
