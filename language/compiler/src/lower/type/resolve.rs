use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::{FunctionContext, LocalBinding};
use super::ScalarType;

impl FunctionContext<'_> {
    /// Resolve the MIR type for a typed expression.
    ///
    /// This bridges DIR type information to MIR types during value lowering.
    /// For scalar types, returns cached primitive types directly.
    /// For aggregate types, looks up the type in the type cache.
    pub(crate) fn mir_type_for_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // first try scalar types
        if let Some(scalar_type) = self.scalar_type_for_expression(expression_id) {
            return match scalar_type {
                ScalarType::Bool => Ok(self.type_lowerer.ty_bool),
                ScalarType::SignedInt { width: 32 } => Ok(self.type_lowerer.ty_i32),
                ScalarType::SignedInt { width: 64 } => Ok(self.type_lowerer.ty_i64),
                ScalarType::Float { width: 32 } => Ok(self.type_lowerer.ty_f32),
                ScalarType::Float { width: 64 } => Ok(self.type_lowerer.ty_f64),
                _ => Err(LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                }),
            };
        }

        // for non-scalar types, check the type cache
        let node = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));
        let type_id = self
            .dir_type_for_expression(expression_id)
            .ok_or(LowerError::MissingType { node })?;

        // direct cache lookup first
        if let Some(&mir_type) = self.type_lowerer.type_cache.get(&type_id) {
            return Ok(mir_type);
        }

        // handle primitive string directly
        let dir_type = self.types.get_type(type_id);
        if matches!(
            dir_type,
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
            }
        ) {
            return self
                .type_lowerer
                .string_type()
                .ok_or(LowerError::MissingType { node });
        }

        // map nominal references to lowered MIR types
        if let dir::Type::Reference { symbol, .. } = dir_type {
            if symbol.ty() == dir::SymbolType::Class {
                return Err(LowerError::MissingType { node });
            }

            if let Some(instance_type_id) = self.types.get_instance_type_id(*symbol)
                && let Some(&mir_type) = self.type_lowerer.type_cache.get(&instance_type_id)
            {
                return Ok(mir_type);
            }
        }

        Err(LowerError::MissingType { node })
    }

    /// Resolve the scalar type for a typed expression.
    pub(crate) fn scalar_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<ScalarType> {
        let type_id = self.dir_type_for_expression(expression_id)?;
        let dir_type = self.types.get_type(type_id);
        self.type_lowerer.scalar_type_for_dir_type(dir_type)
    }

    /// Resolve the DIR type id for a typed expression.
    pub(crate) fn dir_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        let expression = self.dir_tree.get(expression_id);
        let type_id = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => self
                .types
                .get_declared_or_inferred_type_id(expression_id.into_global_any(self.module_id))
                .or_else(|| self.types.get_value_type_id(*target_symbol))
                .or_else(|| self.local_symbol_type_id(*target_symbol)),
            _ => {
                let node_id = expression_id.into_global_any(self.module_id);
                self.types.get_declared_or_inferred_type_id(node_id)
            }
        }?;
        Some(type_id)
    }

    /// Resolve a local symbol to its declared or inferred type.
    fn local_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<dir::LocalTypeId> {
        if symbol_id.module_id != self.module_id {
            return None;
        }

        let symbol = self.symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        self.types
            .get_declared_or_inferred_type_id(primary_declaration)
    }

    /// Resolve a local binding for a symbol reference.
    pub(crate) fn local_binding_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<LocalBinding> {
        let binding = self.locals_by_symbol.get(&target_symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing local reference target symbol".to_string(),
            }
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
        let node_id = expression_id.into_global_any(self.module_id);
        let resolution_id = self.types.get_resolution_for_node(node_id)?;
        Some(self.types.get_resolution(resolution_id))
    }
}
