use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::FunctionLowerer;
use super::ScalarType;
use crate::lower::{GlobalBinding, LocalBinding};

impl FunctionLowerer<'_> {
    /// Create a MissingType error for the given expression.
    pub(crate) fn missing_type_error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerError {
        LowerError::MissingType {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
        }
    }

    /// Create a MissingType error for the given node.
    pub(crate) fn missing_type_error_for_node(&self, node_id: dir::GlobalNodeIdAny) -> LowerError {
        LowerError::MissingType {
            anchor: self.diagnostic_anchor(node_id.into_anchored(Some(self.context.profile))),
        }
    }

    /// Resolve the MIR type for a typed expression.
    ///
    /// This bridges DIR type information to MIR types during value lowering.
    /// For scalar types, returns cached primitive types directly.
    /// For aggregate types, looks up the type in the primitive type cache.
    pub(crate) fn lower_type_for_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the dir type id for the expression
        let type_id = self.type_for_expression_or_error(expression_id)?;
        let node = expression_id.into_global_any(self.context.module_id);

        self.lower_type_id_for_node(type_id, node)
    }

    /// Resolve the MIR type for a type expression.
    pub(crate) fn lower_type_for_type_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the dir type id for the explicit type syntax
        let node = expression_id.into_global_any(self.context.module_id);
        let type_id = self
            .type_id_for_type_expression(expression_id)
            .ok_or_else(|| self.missing_type_error_for_node(node))?;

        self.lower_type_id_for_node(type_id, node)
    }

    /// Resolve the MIR type for a DIR type id.
    pub(crate) fn lower_type_id_for_node(
        &mut self,
        type_id: dir::LocalTypeId,
        node: dir::GlobalNodeIdAny,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // unwrap value wrappers for MIR type lookup
        let type_id = self.unwrap_form_payload_type_id(type_id);

        // return cached types when available
        if let Some(mir_type) = self.context.type_lowerer.cached_type(type_id) {
            return Ok(mir_type);
        }

        // resolve scalar types directly when possible
        let dir_type = self.context.types.get_type(type_id);
        if let Some(scalar_type) = self.context.type_lowerer.scalar_type_for_dir_type(dir_type) {
            return match scalar_type {
                ScalarType::Bool => Some(self.context.type_lowerer.ty_bool),
                ScalarType::SignedInt { width: 32 } => Some(self.context.type_lowerer.ty_i32),
                ScalarType::SignedInt { width: 64 } => Some(self.context.type_lowerer.ty_i64),
                ScalarType::UnsignedInt { width: 32 } => Some(self.context.type_lowerer.ty_u32),
                ScalarType::UnsignedInt { width }
                    if width == self.context.type_lowerer.pointer_width_bits() =>
                {
                    Some(self.context.type_lowerer.ty_usize)
                }
                ScalarType::FLOAT32 => Some(self.context.type_lowerer.ty_f32),
                ScalarType::FLOAT64 => Some(self.context.type_lowerer.ty_f64),
                _ => None,
            }
            .ok_or_else(|| self.missing_type_error_for_node(node));
        }

        // resolve primitive string directly
        if matches!(dir_type, dir::Type::Primitive(dir::PrimitiveType::String)) {
            return self
                .context
                .type_lowerer
                .string_type()
                .ok_or_else(|| self.missing_type_error_for_node(node));
        }

        Err(self.missing_type_error_for_node(node))
    }

    /// Resolve the scalar type for a typed expression.
    pub(crate) fn scalar_type_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ScalarType> {
        let type_id = self.type_for_expression(expression_id)?;
        let type_id = self.unwrap_form_payload_type_id(type_id);

        // handle enum backing scalars
        if let Some(backing) = self.enum_backing_type_for_type(type_id) {
            return self
                .context
                .type_lowerer
                .scalar_type_for_enum_backing(backing);
        }

        let dir_type = self.context.types.get_type(type_id);
        self.context.type_lowerer.scalar_type_for_dir_type(dir_type)
    }

    /// Resolve the DIR type id for a typed expression.
    /// Only accept analysis-provided types.
    pub(crate) fn type_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalTypeId> {
        self.declared_or_inferred_type_id(expression_id)
    }

    /// Resolve a DIR type id for an expression or return MissingType.
    pub(crate) fn type_for_expression_or_error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<dir::LocalTypeId> {
        self.type_for_expression(expression_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
    }

    /// Resolve a signature type id for a node or return MissingType.
    pub(crate) fn signature_type_id_for_node_or_error(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> LowerResult<dir::LocalTypeId> {
        self.context
            .types
            .signature_type_id(node_id)
            .ok_or_else(|| self.missing_type_error_for_node(node_id))
    }

    /// Resolve the declared or inferred type id for an expression.
    fn declared_or_inferred_type_id(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalTypeId> {
        let node_id = expression_id.into_global_any(self.context.module_id);
        self.context.types.get_declared_or_inferred_type_id(node_id)
    }

    /// Resolve the type id encoded in one type expression node.
    pub(crate) fn type_id_for_type_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::LocalTypeId> {
        // read the type expression node
        let expression = self.context.dir_tree.get(expression_id);

        // resolve reference nodes directly through semantic resolution
        if matches!(expression, dir::TypeExpression::Reference { .. }) {
            let node_id = expression_id.into_global_any(self.context.module_id);
            let Some(target_symbol) = self.context.resolutions.symbol_resolution(node_id) else {
                return None;
            };

            if let Some(instance_type_id) = self.context.types.get_instance_type_id(target_symbol) {
                return Some(instance_type_id);
            }

            if let Some(type_id) = self
                .context
                .types
                .symbol_type_id(self.context.symbols, target_symbol)
            {
                return Some(type_id);
            }
        }

        // fall back to the analyzed node type
        let type_id = self.context.types.get_declared_or_inferred_type_id(
            expression_id.into_global_any(self.context.module_id),
        )?;
        match self.context.types.get_type(type_id) {
            dir::Type::Form(value) => Some(value.value),
            dir::Type::Named(reference) => self
                .context
                .types
                .get_instance_type_id(reference.symbol)
                .or(Some(type_id)),
            _ => Some(type_id),
        }
    }

    /// Resolve the class symbol for a DIR type when possible.
    pub(crate) fn class_symbol_for_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        // match the dir type to find a class symbol
        match self.context.types.get_type(type_id) {
            // accept direct class references
            dir::Type::Named(reference)
                if self
                    .context
                    .symbol_is(reference.symbol, dir::SymbolForm::Class) =>
            {
                Some(reference.symbol)
            }
            // unwrap value types
            dir::Type::Form(value) => self.class_symbol_for_type(value.value),
            // search intersection elements
            dir::Type::Intersection(intersection) => intersection
                .elements
                .iter()
                .find_map(|element| self.class_symbol_for_type(*element)),
            // reject non class types
            _ => None,
        }
    }

    /// Strip value wrapper types from a type id.
    pub(crate) fn unwrap_form_payload_type_id(
        &self,
        type_id: dir::LocalTypeId,
    ) -> dir::LocalTypeId {
        // unwrap value type nodes until a concrete type is reached
        let dir_type = self.context.types.get_type(type_id);
        match dir_type {
            dir::Type::Form(value) => self.unwrap_form_payload_type_id(value.value),
            _ => type_id,
        }
    }

    /// Check whether two type ids are equivalent for nominal matching.
    pub(crate) fn type_ids_equivalent(
        &self,
        left_type_id: dir::LocalTypeId,
        right_type_id: dir::LocalTypeId,
    ) -> bool {
        // unwrap value wrappers before comparison
        let left_type_id = self.unwrap_form_payload_type_id(left_type_id);
        let right_type_id = self.unwrap_form_payload_type_id(right_type_id);

        // fast path: structural or nominal equivalence
        if left_type_id == right_type_id {
            return true;
        }

        // match nominal references against their instance types
        let left_instance = match self.context.types.get_type(left_type_id) {
            dir::Type::Named(reference) => {
                self.context.types.get_instance_type_id(reference.symbol)
            }
            _ => None,
        };
        if let Some(left_instance) = left_instance
            && left_instance == right_type_id
        {
            return true;
        }

        // match instance types against nominal references
        let right_instance = match self.context.types.get_type(right_type_id) {
            dir::Type::Named(reference) => {
                self.context.types.get_instance_type_id(reference.symbol)
            }
            _ => None,
        };
        if let Some(right_instance) = right_instance
            && left_type_id == right_instance
        {
            return true;
        }

        false
    }

    /// Resolve the enum backing type for a type id when available.
    fn enum_backing_type_for_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::EnumBackingType> {
        // accept direct enum references
        if let dir::Type::Named(reference) = self.context.types.get_type(type_id)
            && self
                .context
                .symbol_is(reference.symbol, dir::SymbolForm::Enum)
        {
            return self.context.types.get_enum_backing_type(reference.symbol);
        }

        // accept enum instance types
        let symbol = self.context.types.symbol_for_instance_type(type_id)?;
        if !self.context.symbol_is(symbol, dir::SymbolForm::Enum) {
            return None;
        }

        self.context.types.get_enum_backing_type(symbol)
    }

    /// Resolve a local binding for a symbol reference.
    pub(crate) fn local_binding_for_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<LocalBinding> {
        let binding = self
            .state
            .bindings
            .locals_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "missing local reference target symbol".to_string(),
            })?;

        Ok(binding.clone())
    }

    /// Resolve a global binding for a symbol reference.
    pub(crate) fn global_binding_for_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<GlobalBinding> {
        let binding = self
            .context
            .globals_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "unresolved symbol reference".to_string(),
            })?;

        Ok(binding.clone())
    }

    /// Get the call resolution for an expression.
    pub(crate) fn get_call_resolution(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::CallResolution> {
        let node_id = expression_id.into_global_any(self.context.module_id);
        self.context.resolutions.call_resolution(node_id)
    }

    /// Get the member resolution for an expression.
    pub(crate) fn get_member_resolution(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::MemberResolution> {
        let node_id = expression_id.into_global_any(self.context.module_id);
        self.context.resolutions.member_resolution(node_id)
    }
}
