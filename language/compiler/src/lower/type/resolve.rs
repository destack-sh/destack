use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::FunctionContext;
use super::ScalarType;
use crate::lower::item::{GlobalBinding, LocalBinding};

impl FunctionContext<'_> {
    /// Resolve the MIR type for a typed expression.
    ///
    /// This bridges DIR type information to MIR types during value lowering.
    /// For scalar types, returns cached primitive types directly.
    /// For aggregate types, looks up the type in the type cache.
    pub(crate) fn lower_type_for_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the dir type id for the expression
        let type_id = self.type_for_expression_or_error(expression_id)?;

        // unwrap value wrappers for type cache lookup
        let type_id = self.unwrap_value_type_id(type_id);

        // return cached types when available
        if let Some(mir_type) = self.env.type_lowerer.cached_type(type_id) {
            return Ok(mir_type);
        }

        // resolve scalar types directly when possible
        if let Some(scalar_type) = self.scalar_type_for_expression(expression_id) {
            let mir_type = match scalar_type {
                ScalarType::Bool => Some(self.env.type_lowerer.ty_bool),
                ScalarType::SignedInt { width: 32 } => Some(self.env.type_lowerer.ty_i32),
                ScalarType::SignedInt { width: 64 } => Some(self.env.type_lowerer.ty_i64),
                ScalarType::UnsignedInt { width: 32 } => Some(self.env.type_lowerer.ty_u32),
                ScalarType::UnsignedInt { width }
                    if width == self.env.type_lowerer.pointer_width_bits() =>
                {
                    Some(self.env.type_lowerer.ty_usize)
                }
                ScalarType::Float { width: 32 } => Some(self.env.type_lowerer.ty_f32),
                ScalarType::Float { width: 64 } => Some(self.env.type_lowerer.ty_f64),
                _ => None,
            };
            return mir_type.ok_or_else(|| self.missing_type_error(expression_id));
        }

        // resolve primitive string directly
        let dir_type = self.env.types.get_type(type_id);
        if matches!(
            dir_type,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
            }
        ) {
            return self
                .env
                .type_lowerer
                .string_type()
                .ok_or_else(|| self.missing_type_error(expression_id));
        }

        Err(self.missing_type_error(expression_id))
    }

    /// Resolve the scalar type for a typed expression.
    pub(crate) fn scalar_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<ScalarType> {
        let type_id = self.type_for_expression(expression_id)?;
        let type_id = self.unwrap_value_type_id(type_id);

        // handle enum backing scalars
        if let Some(backing) = self.enum_backing_type_for_type(type_id) {
            return self.env.type_lowerer.scalar_type_for_enum_backing(backing);
        }

        let dir_type = self.env.types.get_type(type_id);
        self.env.type_lowerer.scalar_type_for_dir_type(dir_type)
    }

    /// Resolve the DIR type id for a typed expression.
    pub(crate) fn type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        // read the expression node once
        let expression = self.env.dir_tree.get(expression_id);

        // resolve the type id based on the expression kind
        let type_id = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                self.type_id_for_reference_expression(expression_id, *target_symbol)
            }
            Expression::New { left, .. } => self
                .declared_or_inferred_type_id(expression_id)
                .or_else(|| self.type_id_for_constructor_expression(*left)),
            Expression::TaggedScalarExpression { ty, .. }
            | Expression::TaggedTupleExpression { ty, .. }
            | Expression::TaggedObjectExpression { ty, .. } => {
                self.type_id_for_type_expression(*ty)
            }
            Expression::Cast { target_type, .. } => {
                self.type_id_for_cast_expression(expression_id, *target_type)
            }
            _ => self.declared_or_inferred_type_id(expression_id),
        }?;

        Some(type_id)
    }

    /// Resolve the instance type for a constructor callee expression.
    fn type_id_for_constructor_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        // unwrap parenthesized callees
        let mut callee_id = expression_id;
        loop {
            let Expression::Parenthesized { expression } = self.env.dir_tree.get(callee_id) else {
                break;
            };
            callee_id = *expression;
        }

        // resolve nominal instance types from references
        let symbol = match self.env.dir_tree.get(callee_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return None,
        };

        self.env.types.get_instance_type_id(symbol)
    }

    /// Resolve a DIR type id for an expression or return MissingType.
    pub(crate) fn type_for_expression_or_error(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<dir::LocalTypeId> {
        self.type_for_expression(expression_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
    }

    /// Resolve the declared or inferred type id for an expression.
    fn declared_or_inferred_type_id(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        let node_id = expression_id.into_global_any(self.env.module_id);
        self.env.types.get_declared_or_inferred_type_id(node_id)
    }

    /// Resolve the type id for a reference expression.
    fn type_id_for_reference_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        self.declared_or_inferred_type_id(expression_id)
            .or_else(|| {
                self.env
                    .types
                    .get_type_id_for_symbol(self.env.symbols, target_symbol)
            })
    }

    /// Resolve the type id for a cast expression.
    fn type_id_for_cast_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_type: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        // prefer declared or inferred types for the expression
        self.declared_or_inferred_type_id(expression_id)
            .or_else(|| self.type_id_for_type_expression(target_type))
    }

    /// Resolve the type id encoded in a type expression node.
    pub(crate) fn type_id_for_type_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        // read the type expression node
        let expression = self.env.dir_tree.get(expression_id);

        // use the explicit type id when available
        if let Expression::Type { value } = expression {
            return Some(*value);
        }

        // resolve symbol references directly for type expressions
        if let Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. } = expression
        {
            if let Some(instance_type_id) = self.env.types.get_instance_type_id(*target_symbol) {
                return Some(instance_type_id);
            }

            if let Some(type_id) = self
                .env
                .types
                .get_type_id_for_symbol(self.env.symbols, *target_symbol)
            {
                return Some(type_id);
            }
        }

        // fall back to declared or inferred type ids
        let type_id = self
            .env
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(self.env.module_id))?;
        match self.env.types.get_type(type_id) {
            dir::Type::Value { value } => Some(*value),
            _ => Some(type_id),
        }
    }

    /// Resolve the class symbol for a DIR type when possible.
    pub(crate) fn class_symbol_for_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // match the dir type to find a class symbol
        match self.env.types.get_type(type_id) {
            // accept direct class references
            dir::Type::Reference { symbol, .. } if symbol.ty() == dir::SymbolType::Class => {
                Some(*symbol)
            }
            // unwrap value types
            dir::Type::Value { value } => self.class_symbol_for_type(*value),
            // search intersection elements
            dir::Type::Intersection { elements } => elements
                .iter()
                .find_map(|element| self.class_symbol_for_type(*element)),
            // reject non class types
            _ => None,
        }
    }

    /// Strip value wrapper types from a type id.
    pub(crate) fn unwrap_value_type_id(&self, type_id: dir::LocalTypeId) -> dir::LocalTypeId {
        // unwrap value type nodes until a concrete type is reached
        let dir_type = self.env.types.get_type(type_id);
        match dir_type {
            dir::Type::Value { value } => self.unwrap_value_type_id(*value),
            _ => type_id,
        }
    }

    /// Resolve the enum backing type for a type id when available.
    fn enum_backing_type_for_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::EnumBackingType> {
        // accept direct enum references
        if let dir::Type::Reference { symbol, .. } = self.env.types.get_type(type_id)
            && symbol.ty() == dir::SymbolType::Enum
        {
            return self.env.types.get_enum_backing_type(*symbol);
        }

        // accept enum instance types
        let symbol = self.env.types.symbol_for_instance_type(type_id)?;
        if symbol.ty() != dir::SymbolType::Enum {
            return None;
        }

        self.env.types.get_enum_backing_type(symbol)
    }

    /// Resolve a local binding for a symbol reference.
    pub(crate) fn local_binding_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<LocalBinding> {
        let binding = self
            .state
            .bindings
            .locals_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing local reference target symbol".to_string(),
            })?;

        Ok(*binding)
    }

    /// Resolve a global binding for a symbol reference.
    pub(crate) fn global_binding_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<GlobalBinding> {
        let binding = self
            .env
            .globals_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "unresolved symbol reference".to_string(),
            })?;

        Ok(*binding)
    }

    /// Get the resolution for an expression from the TypeTable.
    ///
    /// Returns the Resolution if one is attached to this expression, or None.
    pub(crate) fn get_resolution(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<&Resolution> {
        let node_id = expression_id.into_global_any(self.env.module_id);
        let resolution_id = self.env.types.get_resolution_for_node(node_id)?;
        Some(self.env.types.get_resolution(resolution_id))
    }
}
