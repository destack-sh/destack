use destack_dir as dir;
use destack_source::ModuleId;
use dir::{
    Block, Expression, LocalNodeId, LocalNodeIdAny, LocalTypeId, PrimitiveType, RuntimeCheckKind,
    ScalarLiteral, SymbolType, Type, TypeLiteral, TypeTable,
};

use crate::{Compiler, ElaborateError, ElaborateResult};

impl Compiler {
    /// Record an inferred type for a synthesized expression.
    pub(crate) fn set_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
    ) {
        types.set_inferred_type(expression_id.into_global_any(module_id), type_id);
    }

    /// Record a boolean type for a synthesized expression.
    pub(crate) fn set_boolean_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let bool_type = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        };
        let bool_type_id = types.insert_type_from(bool_type, expression_id);
        self.set_expression_type(types, module_id, expression_id, bool_type_id);
    }

    /// Record a scalar literal type for a synthesized literal expression.
    pub(crate) fn set_scalar_literal_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        literal: ScalarLiteral,
    ) {
        let literal_type = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        };
        let literal_type_id = types.insert_type_from(literal_type, expression_id);
        self.set_expression_type(types, module_id, expression_id, literal_type_id);
    }

    /// Record a void type for a synthesized expression.
    pub(crate) fn set_void_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types, expression_id.into_any());
        self.set_expression_type(types, module_id, expression_id, void_type_id);
    }

    /// Record a never type for a synthesized expression.
    pub(crate) fn set_never_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let never_type_id = self.never_type_id(types, expression_id.into_any());
        self.set_expression_type(types, module_id, expression_id, never_type_id);
    }

    /// Record a void type for a block expression wrapper.
    pub(crate) fn set_void_block_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        block_id: LocalNodeId<Block>,
        expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types, block_id.into_any());
        types.set_inferred_type(block_id.into_global_any(module_id), void_type_id);
        self.set_expression_type(types, module_id, expression_id, void_type_id);
    }

    /// Resolve a symbol value type id or return an error.
    pub(crate) fn value_type_id_or_error(
        &self,
        module_id: ModuleId,
        symbol: dir::GlobalSymbolId,
        node_id: LocalNodeIdAny,
        types: &TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        types
            .get_value_type_id(symbol)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: node_id.into_global(module_id).into_anchored(None),
            })
    }

    /// Resolve an expression type id or return an error.
    pub(crate) fn expression_type_id_or_error(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: expression_id.into_global_any(module_id).into_anchored(None),
            })
    }

    /// Allocate a void type id for a synthesized node.
    pub(crate) fn void_type_id(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        types.insert_type_from_any(ty, node_id)
    }

    /// Allocate a never type id for a synthesized node.
    pub(crate) fn never_type_id(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        types.insert_type_from_any(ty, node_id)
    }

    /// Update the inferred type for a block after rewriting expressions.
    pub(crate) fn reinfer_block_type(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &dir::Tree,
        types: &mut TypeTable,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        let block = tree.get(block_id);
        let block_type_id = if let Some(last_expression_id) = block.last_expression() {
            self.expression_type_id_or_error(module_id, last_expression_id, types)?
        } else {
            self.void_type_id(types, block_id.into_any())
        };

        types.set_inferred_type(block_id.into_global_any(module_id), block_type_id);
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            let Expression::Block(block) = tree.get(expression_id) else {
                continue;
            };
            if *block == block_id {
                self.set_expression_type(types, module_id, expression_id, block_type_id);
            }
        }

        Ok(())
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements } => flattened.extend(elements.iter().copied()),
                _ => flattened.push(element_id),
            }
        }

        flattened.sort_unstable_by_key(|id| id.0);
        flattened.dedup();

        match flattened.len() {
            0 => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                },
                types.get_type_source(source_type_id),
            ),
            1 => flattened[0],
            _ => types.insert_type_from_any(
                Type::Union {
                    elements: flattened,
                },
                types.get_type_source(source_type_id),
            ),
        }
    }

    /// Determine the runtime check kind for a type guard relation.
    pub(crate) fn runtime_check_kind_for_relation(
        &self,
        types: &TypeTable,
        value_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
    ) -> Option<RuntimeCheckKind> {
        let value_type_id = types.unwrap_value_type_id(value_type_id);
        let target_type_id = types.unwrap_value_type_id(target_type_id);

        // identical ids need no runtime check
        if value_type_id == target_type_id {
            return Some(RuntimeCheckKind::Constant(true));
        }

        // only runtime visible targets are currently checkable
        if !is_runtime_checkable_target(types, target_type_id) {
            return None;
        }

        runtime_check_kind_for_value(types, value_type_id)
    }
}

/// Return whether one target type can be checked at runtime.
fn is_runtime_checkable_target(types: &TypeTable, type_id: LocalTypeId) -> bool {
    match types.get_type(type_id) {
        Type::Union { elements } => elements
            .iter()
            .copied()
            .all(|element| is_runtime_checkable_target(types, types.unwrap_value_type_id(element))),
        Type::Reference { symbol, .. } => matches!(
            symbol.local_id.ty,
            SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype
        ),
        _ => false,
    }
}

/// Return the runtime identity carried by one value type.
fn runtime_check_kind_for_value(
    types: &TypeTable,
    type_id: LocalTypeId,
) -> Option<RuntimeCheckKind> {
    match types.get_type(type_id) {
        Type::Union { .. } => Some(RuntimeCheckKind::UnionTag),
        Type::Reference { symbol, .. } => match symbol.local_id.ty {
            SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype => {
                Some(RuntimeCheckKind::TypeDescriptor)
            }
            _ => None,
        },
        Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        } => Some(RuntimeCheckKind::TypeDescriptor),
        Type::Value { value } => {
            runtime_check_kind_for_value(types, types.unwrap_value_type_id(*value))
        }
        _ => None,
    }
}
