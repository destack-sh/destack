use destack_dir as dir;
use destack_dir::GuardEntry;
use destack_source::ModuleId;
use dir::{
    Block, Expression, LocalNodeId, LocalNodeIdAny, LocalTypeId, PrimitiveType, ScalarLiteral,
    Type, TypeLiteral, TypeSegment, TypeTable,
};

use crate::{Compiler, ElaborateError, ElaborateResult};

impl Compiler {
    /// Record an inferred type for a synthesized expression.
    pub(crate) fn set_expression_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
    ) {
        types_tail.set_inferred_type(expression_id.into_global_any(module_id), type_id);
    }

    /// Record a boolean type for a synthesized expression.
    pub(crate) fn set_boolean_expression_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let bool_type = Type::Literal(dir::LiteralType {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        });
        let bool_type_id = types_tail.insert_type_from(bool_type, expression_id);
        self.set_expression_type(types_tail, module_id, expression_id, bool_type_id);
    }

    /// Record a scalar literal type for a synthesized literal expression.
    pub(crate) fn set_scalar_literal_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        literal: ScalarLiteral,
    ) {
        let literal_type = Type::Literal(dir::LiteralType {
            value: TypeLiteral::ScalarLiteral(literal),
        });
        let literal_type_id = types_tail.insert_type_from(literal_type, expression_id);
        self.set_expression_type(types_tail, module_id, expression_id, literal_type_id);
    }

    /// Record a void type for a synthesized expression.
    pub(crate) fn set_void_expression_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types_tail, expression_id.into_any());
        self.set_expression_type(types_tail, module_id, expression_id, void_type_id);
    }

    /// Record a never type for a synthesized expression.
    pub(crate) fn set_never_expression_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let never_type_id = self.never_type_id(types_tail, expression_id.into_any());
        self.set_expression_type(types_tail, module_id, expression_id, never_type_id);
    }

    /// Record a void type for a block expression wrapper.
    pub(crate) fn set_void_block_expression_type(
        &self,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
        block_id: LocalNodeId<Block>,
        expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types_tail, block_id.into_any());
        types_tail.set_inferred_type(block_id.into_global_any(module_id), void_type_id);
        self.set_expression_type(types_tail, module_id, expression_id, void_type_id);
    }

    /// Resolve an expression type id or return an error.
    pub(crate) fn expression_type_id_or_error(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable<'_>,
    ) -> ElaborateResult<LocalTypeId> {
        types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                anchor: module_id.into(),
            })
    }

    /// Allocate a void type id for a synthesized node.
    pub(crate) fn void_type_id(
        &self,
        types_tail: &mut TypeSegment,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::Literal(dir::LiteralType {
            value: TypeLiteral::Void,
        });
        types_tail.insert_type_from_any(ty, node_id)
    }

    /// Allocate a never type id for a synthesized node.
    pub(crate) fn never_type_id(
        &self,
        types_tail: &mut TypeSegment,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::Literal(dir::LiteralType {
            value: TypeLiteral::Never,
        });
        types_tail.insert_type_from_any(ty, node_id)
    }

    /// Update the inferred type for a block after rewriting expressions.
    pub(crate) fn reinfer_block_type(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &dir::Tree,
        types: &TypeTable<'_>,
        types_tail: &mut TypeSegment,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        let block = tree.get(block_id);
        let block_type_id = if let Some(last_expression_id) = block.last_expression() {
            let types = types.with_tail(types_tail);

            self.expression_type_id_or_error(module_id, last_expression_id, &types)?
        } else {
            self.void_type_id(types_tail, block_id.into_any())
        };

        types_tail.set_inferred_type(block_id.into_global_any(module_id), block_type_id);
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            let Expression::Block(block) = tree.get(expression_id) else {
                continue;
            };
            if *block == block_id {
                self.set_expression_type(types_tail, module_id, expression_id, block_type_id);
            }
        }

        Ok(())
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &TypeTable<'_>,
        types_tail: &mut TypeSegment,
    ) -> LocalTypeId {
        let mut flattened = Vec::new();
        let types = types.with_tail(types_tail);
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union(union) => flattened.extend(union.elements.iter().copied()),
                _ => flattened.push(element_id),
            }
        }

        flattened.sort_unstable_by_key(|id| id.0);
        flattened.dedup();
        let source_id = types.get_type_source(source_type_id);
        drop(types);

        match flattened.len() {
            0 => types_tail.insert_type_from_any(
                Type::Literal(dir::LiteralType {
                    value: TypeLiteral::Never,
                }),
                source_id,
            ),
            1 => flattened[0],
            _ => types_tail.insert_type_from_any(
                Type::Union(dir::UnionType {
                    elements: flattened,
                }),
                source_id,
            ),
        }
    }

    /// Determine the runtime check kind for a type guard relation.
    pub(crate) fn guard_entry_for_relation(
        &self,
        types: &TypeTable<'_>,
        value_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
    ) -> Option<GuardEntry> {
        let value_type_id = types.unwrap_value_type_id(value_type_id);
        let target_type_id = types.unwrap_value_type_id(target_type_id);

        // identical ids need no runtime check
        if value_type_id == target_type_id {
            return Some(GuardEntry::Constant(true));
        }

        // only runtime visible targets are currently checkable
        if !is_runtime_checkable_target(types, target_type_id) {
            return None;
        }

        guard_entry_for_value(types, value_type_id)
    }
}

/// Return whether one target type can be checked at runtime.
fn is_runtime_checkable_target(types: &TypeTable<'_>, type_id: LocalTypeId) -> bool {
    match types.get_type(type_id) {
        Type::Union(union) => {
            union.elements.iter().copied().all(|element| {
                is_runtime_checkable_target(types, types.unwrap_value_type_id(element))
            })
        }
        Type::Reference(_) => true,
        _ => false,
    }
}

/// Return the runtime identity carried by one value type.
fn guard_entry_for_value(types: &TypeTable<'_>, type_id: LocalTypeId) -> Option<GuardEntry> {
    match types.get_type(type_id) {
        Type::Union(_) => Some(GuardEntry::UnionTag),
        Type::Reference(_) => Some(GuardEntry::TypeDescriptor),
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Unknown,
        }) => Some(GuardEntry::TypeDescriptor),
        Type::Value(value) => guard_entry_for_value(types, types.unwrap_value_type_id(value.value)),
        _ => None,
    }
}
