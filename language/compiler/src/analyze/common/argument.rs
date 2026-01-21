use std::collections::HashSet;

use destack_dir::{Argument, Expression, GlobalSymbolId, LocalNodeId, NodeTree, Property};

use crate::Compiler;

/// Track static parameter references in type and value positions.
#[derive(Debug, Default, Clone)]
pub(crate) struct StaticParameterReferences {
    /// Static parameters referenced in type positions.
    pub(crate) type_symbols: HashSet<GlobalSymbolId>,
    /// Static parameters referenced in value positions.
    pub(crate) value_symbols: HashSet<GlobalSymbolId>,
}

/// Control how reference positions are recorded during traversal.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StaticParameterReferencePosition {
    /// Record references as type usage.
    Type,
    /// Record references as value usage.
    Value,
}

impl Compiler {
    /// Collect static parameter references from an expression tree.
    pub(crate) fn collect_static_parameter_references_from_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        references: &mut StaticParameterReferences,
        position: StaticParameterReferencePosition,
    ) {
        let expression = tree.get(expression_id);

        // record direct reference usage
        if let Expression::LocalReference {
            target_symbol,
            static_arguments,
            ..
        }
        | Expression::ModuleReference {
            target_symbol,
            static_arguments,
            ..
        }
        | Expression::GlobalReference {
            target_symbol,
            static_arguments,
            ..
        } = expression
        {
            match position {
                StaticParameterReferencePosition::Type => {
                    references.type_symbols.insert(*target_symbol);
                }
                StaticParameterReferencePosition::Value => {
                    references.value_symbols.insert(*target_symbol);
                }
            }

            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    self.collect_static_parameter_references_from_argument(
                        tree,
                        *argument_id,
                        references,
                    );
                }
            }

            return;
        }

        // traverse child expressions
        match expression {
            Expression::TypeUnary { right, .. }
            | Expression::Unary { right, .. }
            | Expression::ValueOf { right, .. }
            | Expression::ReferenceOf { right, .. }
            | Expression::PointerOf { right, .. }
            | Expression::Must { left: right } => {
                self.collect_static_parameter_references_from_expression(
                    tree, *right, references, position,
                );
            }
            Expression::TypeBinary { left, right, .. } | Expression::Binary { left, right, .. } => {
                self.collect_static_parameter_references_from_expression(
                    tree, *left, references, position,
                );
                self.collect_static_parameter_references_from_expression(
                    tree, *right, references, position,
                );
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_static_parameter_references_from_expression(
                    tree, *left, references, position,
                );
                self.collect_static_parameter_references_from_expression(
                    tree, *right, references, position,
                );
                self.collect_static_parameter_references_from_expression(
                    tree, *then_type, references, position,
                );
                self.collect_static_parameter_references_from_expression(
                    tree, *else_type, references, position,
                );
            }
            Expression::TypeMapped {
                parameter, value, ..
            } => {
                self.collect_static_parameter_references_from_expression(
                    tree,
                    parameter.constraint,
                    references,
                    position,
                );
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_static_parameter_references_from_expression(
                        tree, key_remap, references, position,
                    );
                }
                self.collect_static_parameter_references_from_expression(
                    tree, *value, references, position,
                );
            }
            Expression::TypeIndex { left, index: _ } => {
                // only record the left side, index position is handled during type evaluation
                self.collect_static_parameter_references_from_expression(
                    tree, *left, references, position,
                );
            }
            Expression::TypeTemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_static_parameter_references_from_expression(
                        tree, *span, references, position,
                    );
                }
            }
            Expression::TypeImport {
                static_arguments, ..
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument_id in static_arguments {
                        self.collect_static_parameter_references_from_argument(
                            tree,
                            *argument_id,
                            references,
                        );
                    }
                }
            }
            Expression::TypeInfer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.collect_static_parameter_references_from_expression(
                        tree,
                        *constraint,
                        references,
                        position,
                    );
                }
            }
            Expression::TypePredicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_static_parameter_references_from_expression(
                        tree, *target, references, position,
                    );
                }
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                for element_id in elements {
                    self.collect_static_parameter_references_from_argument(
                        tree,
                        *element_id,
                        references,
                    );
                }
            }
            Expression::ObjectExpression { properties } => {
                for property_id in properties {
                    self.collect_static_parameter_references_from_property(
                        tree,
                        *property_id,
                        references,
                        position,
                    );
                }
            }
            Expression::Index { left, right } => {
                self.collect_static_parameter_references_from_expression(
                    tree, *left, references, position,
                );
                if let Some(right) = right {
                    self.collect_static_parameter_references_from_expression(
                        tree,
                        *right,
                        references,
                        StaticParameterReferencePosition::Value,
                    );
                }
            }
            Expression::Parenthesized { expression } => {
                self.collect_static_parameter_references_from_expression(
                    tree,
                    *expression,
                    references,
                    position,
                );
            }
            _ => {}
        }
    }

    /// Collect static parameter references from a static argument expression.
    pub(crate) fn collect_static_parameter_references_from_argument(
        &self,
        tree: &NodeTree,
        argument_id: LocalNodeId<Argument>,
        references: &mut StaticParameterReferences,
    ) {
        // resolve the argument expression
        let argument = tree.get(argument_id);
        let expression_id = argument.value();
        self.collect_static_parameter_references_from_expression(
            tree,
            expression_id,
            references,
            StaticParameterReferencePosition::Type,
        );
    }

    /// Collect static parameter references from an object property expression.
    pub(crate) fn collect_static_parameter_references_from_property(
        &self,
        tree: &NodeTree,
        property_id: LocalNodeId<Property>,
        references: &mut StaticParameterReferences,
        position: StaticParameterReferencePosition,
    ) {
        // inspect property type positions
        let property = tree.get(property_id);

        match property {
            Property::Field { value, default, .. } => {
                if let Some(value) = value {
                    self.collect_static_parameter_references_from_expression(
                        tree, *value, references, position,
                    );
                }
                if let Some(default) = default {
                    self.collect_static_parameter_references_from_expression(
                        tree,
                        *default,
                        references,
                        StaticParameterReferencePosition::Value,
                    );
                }
            }
            Property::Method { signature, .. } => {
                if let Some(return_type) = signature.return_type {
                    self.collect_static_parameter_references_from_expression(
                        tree,
                        return_type,
                        references,
                        position,
                    );
                }
            }
            Property::Spread { value, .. } => {
                self.collect_static_parameter_references_from_expression(
                    tree, *value, references, position,
                );
            }
        }
    }
}
