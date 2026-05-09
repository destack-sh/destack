use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};
use destack_ast::{self as ast, LocalNodeId, Tree, TypeExpression};
use destack_workspace::LintSeverity;

declare_lint! {
    /// Warn on overly complex type expressions.
    ///
    /// Deeply nested generic types and large type compositions are hard to read.
    /// Consider introducing named type aliases for complex shapes.
    #[lint(
        id = "no-complex-type",
        code = "LX016",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoComplexType,
    "Warn on overly complex types"
}

impl LintRule for NoComplexType {
    fn meta(&self) -> &'static LintMeta {
        NoComplexType::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_type_complexity = ctx.options.complexity.max_type_complexity;

        // check only top level type annotation roots
        for type_expression_id in ctx.tree.iter_nodes::<ast::TypeExpression>() {
            if has_type_expression_parent(ctx, type_expression_id) {
                continue;
            }

            // compute structural complexity score for this type expression
            let complexity = type_expression_complexity(ctx.tree, type_expression_id);
            if complexity <= max_type_complexity {
                continue;
            }

            // resolve effective severity
            let severity = ctx.get_effective_severity(meta, type_expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one complex type diagnostic
            ctx.report(
                LintReport::new(
                    NO_COMPLEX_TYPE.id,
                    NO_COMPLEX_TYPE.code,
                    NO_COMPLEX_TYPE.category,
                    severity,
                    format!("type has complexity {complexity} (max {max_type_complexity})"),
                    ctx.tree.get_span(type_expression_id),
                )
                .label("consider extracting a named type alias"),
            );
        }
    }
}

/// Return true when one type expression has a parent type expression.
fn has_type_expression_parent(
    ctx: &LintAstContext<'_>,
    type_expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(type_expression_id) else {
        return false;
    };

    ctx.tree.get_node_type(parent_id) == ast::NodeType::TypeExpression
}

/// Compute one nesting style complexity score for a type expression.
fn type_expression_complexity(
    tree: &Tree,
    type_expression_id: LocalNodeId<TypeExpression>,
) -> usize {
    type_expression_complexity_inner(tree, type_expression_id, 0)
}

/// Compute the maximum type nesting depth below one type expression.
fn type_expression_complexity_inner(
    tree: &Tree,
    type_expression_id: LocalNodeId<TypeExpression>,
    current_depth: usize,
) -> usize {
    let type_expression = tree.get(type_expression_id);
    let increases_depth = matches!(
        type_expression,
        TypeExpression::Reference {
            generic_arguments,
            ..
        } if !generic_arguments.is_empty()
    ) || matches!(
        type_expression,
        TypeExpression::Member {
            generic_arguments,
            ..
        } if !generic_arguments.is_empty()
    ) || matches!(
        type_expression,
        TypeExpression::Union { .. }
            | TypeExpression::Intersection { .. }
            | TypeExpression::Readonly { .. }
            | TypeExpression::KeyOf { .. }
            | TypeExpression::TypeOfValue { .. }
            | TypeExpression::Must { .. }
            | TypeExpression::AsComptime { .. }
            | TypeExpression::Not { .. }
            | TypeExpression::OwnedOf { .. }
            | TypeExpression::BorrowedOf { .. }
            | TypeExpression::PointerOf { .. }
            | TypeExpression::Conditional { .. }
            | TypeExpression::Mapped { .. }
            | TypeExpression::Index { .. }
            | TypeExpression::TemplateLiteral { .. }
            | TypeExpression::Tuple { .. }
            | TypeExpression::ArrayTuple { .. }
            | TypeExpression::Object { .. }
            | TypeExpression::Array { .. }
            | TypeExpression::Slice { .. }
            | TypeExpression::FixedArray { .. }
    );
    let current_depth = if increases_depth {
        current_depth + 1
    } else {
        current_depth
    };
    let mut max_depth = current_depth;

    match type_expression {
        TypeExpression::Parenthesized { expression } => {
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *expression,
                current_depth,
            ));
        }
        TypeExpression::Array { element } | TypeExpression::Slice { element } => {
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *element,
                current_depth,
            ));
        }
        TypeExpression::FixedArray { element, length } => {
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *element,
                current_depth,
            ));
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *length,
                current_depth,
            ));
        }
        TypeExpression::Object { members } => {
            for member_id in members {
                let member = tree.get(*member_id);
                match member {
                    ast::TypeMember::Field {
                        declared_type: Some(declared_type),
                        ..
                    } => {
                        max_depth = max_depth.max(type_expression_complexity_inner(
                            tree,
                            *declared_type,
                            current_depth,
                        ));
                    }
                    ast::TypeMember::Field {
                        declared_type: None,
                        ..
                    } => {}
                    ast::TypeMember::Method { signature, .. } => {
                        if let Some(return_type) = signature.return_type {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                return_type,
                                current_depth,
                            ));
                        }
                    }
                    ast::TypeMember::CallSignature { signature } => {
                        if let Some(return_type) = signature.return_type {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                return_type,
                                current_depth,
                            ));
                        }
                    }
                    ast::TypeMember::ConstructSignature { signature } => {
                        if let Some(return_type) = signature.return_type {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                return_type,
                                current_depth,
                            ));
                        }
                    }
                    ast::TypeMember::IndexSignature {
                        key_type,
                        value_type,
                        ..
                    } => {
                        max_depth = max_depth.max(type_expression_complexity_inner(
                            tree,
                            *key_type,
                            current_depth,
                        ));
                        max_depth = max_depth.max(type_expression_complexity_inner(
                            tree,
                            *value_type,
                            current_depth,
                        ));
                    }
                    ast::TypeMember::Embed { value } => {
                        max_depth = max_depth.max(type_expression_complexity_inner(
                            tree,
                            *value,
                            current_depth,
                        ));
                    }
                    ast::TypeMember::AssociatedType {
                        constraint, value, ..
                    } => {
                        if let Some(constraint) = constraint {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                *constraint,
                                current_depth,
                            ));
                        }

                        if let Some(value) = value {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                *value,
                                current_depth,
                            ));
                        }
                    }
                    ast::TypeMember::AssociatedConst { declared_type, .. } => {
                        if let Some(declared_type) = declared_type {
                            max_depth = max_depth.max(type_expression_complexity_inner(
                                tree,
                                *declared_type,
                                current_depth,
                            ));
                        }
                    }
                    ast::TypeMember::Error => {}
                }
            }
        }
        TypeExpression::Member { left, .. }
        | TypeExpression::Readonly { target_type: left }
        | TypeExpression::KeyOf { target_type: left }
        | TypeExpression::Must { target_type: left }
        | TypeExpression::AsComptime { target_type: left }
        | TypeExpression::Not { target_type: left }
        | TypeExpression::OwnedOf {
            target_type: left, ..
        }
        | TypeExpression::BorrowedOf {
            target_type: left, ..
        }
        | TypeExpression::PointerOf {
            target_type: left, ..
        } => {
            max_depth = max_depth.max(type_expression_complexity_inner(tree, *left, current_depth));
        }
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            for element_id in elements {
                max_depth = max_depth.max(type_expression_complexity_inner(
                    tree,
                    *element_id,
                    current_depth,
                ));
            }
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            max_depth = max_depth.max(type_expression_complexity_inner(tree, *left, current_depth));
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *extends_type,
                current_depth,
            ));
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *then_type,
                current_depth,
            ));
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *else_type,
                current_depth,
            ));
        }
        TypeExpression::Mapped {
            parameter, value, ..
        } => {
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                parameter.source_type,
                current_depth,
            ));
            if let Some(key_remap) = parameter.key_remap {
                max_depth = max_depth.max(type_expression_complexity_inner(
                    tree,
                    key_remap,
                    current_depth,
                ));
            }
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *value,
                current_depth,
            ));
        }
        TypeExpression::Index { left, index } => {
            max_depth = max_depth.max(type_expression_complexity_inner(tree, *left, current_depth));
            max_depth = max_depth.max(type_expression_complexity_inner(
                tree,
                *index,
                current_depth,
            ));
        }
        TypeExpression::TemplateLiteral { spans, .. } => {
            for span_id in spans {
                max_depth = max_depth.max(type_expression_complexity_inner(
                    tree,
                    *span_id,
                    current_depth,
                ));
            }
        }
        TypeExpression::Infer { constraint, .. } => {
            if let Some(constraint) = constraint {
                max_depth = max_depth.max(type_expression_complexity_inner(
                    tree,
                    *constraint,
                    current_depth,
                ));
            }
        }
        TypeExpression::Predicate { target, .. } => {
            if let Some(target) = target {
                max_depth = max_depth.max(type_expression_complexity_inner(
                    tree,
                    *target,
                    current_depth,
                ));
            }
        }
        TypeExpression::Tuple { elements } | TypeExpression::ArrayTuple { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                match element {
                    ast::TupleElement::Element { value, .. }
                    | ast::TupleElement::Spread { value, .. } => {
                        max_depth = max_depth.max(type_expression_complexity_inner(
                            tree,
                            *value,
                            current_depth,
                        ));
                    }
                    ast::TupleElement::Error => {}
                }
            }
        }
        TypeExpression::ScalarLiteral { .. }
        | TypeExpression::Literal { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::Declaration { .. }
        | TypeExpression::FunctionTypeDeclaration(_)
        | TypeExpression::ConstructorTypeDeclaration(_)
        | TypeExpression::Reference { .. }
        | TypeExpression::Const
        | TypeExpression::This
        | TypeExpression::TypeOfValue { .. }
        | TypeExpression::Missing
        | TypeExpression::Error => {}
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_complex_type_alias() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.complexity.max_type_complexity = 3);
        let result = test.lint_ast(
            "no_complex_type/test_detects_complex_type_alias.ds",
            r#"
type Value = Array<Map<string, List<Set<int32>>>>;
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_allows_simple_type_annotation() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType);
        let result = test.lint_ast(
            "no_complex_type/test_allows_simple_type_annotation.ds",
            r#"
let value: Array<string>;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }

    #[test]
    fn test_detects_complex_parameter_type() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.complexity.max_type_complexity = 2);
        let result = test.lint_ast(
            "no_complex_type/test_detects_complex_parameter_type.ds",
            r#"
function run(value: Array<Map<string, Set<int32>>>): void {}
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_ignores_runtime_expression_complexity() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.complexity.max_type_complexity = 1);
        let result = test.lint_ast(
            "no_complex_type/test_ignores_runtime_expression_complexity.ds",
            r#"
let value = a | b | c | d;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }
}
