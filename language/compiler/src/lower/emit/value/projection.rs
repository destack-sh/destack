use destack_base::StringId;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution};
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a member access expression to a field_get.
    pub(crate) fn lower_member_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // handle getter access using resolution
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && matches!(
                self.member_mode_for_symbol(target_symbol),
                Some(destack_dir::FunctionMode::Getter)
            )
        {
            return self.lower_getter_call(expression_id, left_id, target_symbol);
        }

        // lower the aggregate value
        let (mut aggregate_value, aggregate_type) = self.lower_value_expression(left_id)?;

        // resolve field index through the type lowerer
        let field_index = self
            .env
            .type_lowerer
            .field_index_for_type(
                aggregate_type,
                field_name,
                self.env.strings,
                self.state.builder.tree(),
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "field not found in aggregate type".to_string(),
            })?;

        // ensure constructor fields are initialized before read
        if matches!(self.env.dir_tree.get(left_id), Expression::This) {
            let node = expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile));
            self.ensure_constructor_field_initialized(node, field_index as u32, field_name)?;
        }

        // load through references before field access
        let mut aggregate_type = aggregate_type;
        loop {
            let aggregate_mir_type = self.state.builder.tree().get(aggregate_type).clone();
            match aggregate_mir_type {
                mir::Type::Reference { pointee, .. } => {
                    aggregate_value = self.state.builder.load(aggregate_value, pointee);
                    aggregate_type = pointee;
                }
                _ => break,
            }
        }

        // get the result type
        let result_type = self.mir_type_for_expression(expression_id)?;

        // emit field_get
        let value = self
            .state
            .builder
            .field_get(aggregate_value, field_index as u32);
        Ok((value, result_type))
    }

    /// Resolve a static member symbol for a member access expression.
    pub(crate) fn resolved_member_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // read the resolution from analyze
        let resolution = self.get_resolution(expression_id)?;

        match resolution {
            Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
            _ => None,
        }
    }

    /// Resolve the function mode for a member symbol when available.
    pub(crate) fn member_mode_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<destack_dir::FunctionMode> {
        // load the module for this symbol
        let module = self.env.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.env.profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // resolve the primary declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.primary_declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // handle member declarations
        if let Ok(member_id) = primary.local_id.try_into_typed::<destack_dir::Member>() {
            let member = tree.get(member_id);
            if let destack_dir::Member::Method { signature, .. } = member {
                return signature.mode;
            }
        }

        // handle property declarations
        if let Ok(property_id) = primary.local_id.try_into_typed::<destack_dir::Property>() {
            let property = tree.get(property_id);
            if let destack_dir::Property::Method { signature, .. } = property {
                return signature.mode;
            }
        }

        None
    }

    /// Lower a getter call for a resolved member access.
    pub(crate) fn lower_getter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the receiver value
        let receiver_value = if self.receiver_is_namespace_reference(receiver_id) {
            None
        } else {
            let (value, _) = self.lower_value_expression(receiver_id)?;
            Some(value)
        };

        // get the result type
        let result_type = self.mir_type_for_expression(expression_id)?;

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = receiver_value {
            arguments.push(receiver);
        }

        // use interface dispatch metadata when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (self.dir_type_for_expression(receiver_id), receiver_value)
            && let Some((function_id, metadata)) = self.interface_dispatch_target(
                expression_id,
                receiver_type_id,
                receiver_value,
                target_symbol,
                result_type,
            )?
        {
            let value = self
                .state
                .builder
                .call_with_metadata(function_id, arguments, metadata)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "getter call returned no value".to_string(),
                })?;
            return Ok((value, result_type));
        }

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // emit the call
        let value = self
            .state
            .builder
            .call(function_id, arguments)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "getter call returned no value".to_string(),
            })?;

        Ok((value, result_type))
    }

    /// Lower a setter call for a resolved member assignment.
    pub(crate) fn lower_setter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        value: mir::Value,
    ) -> LowerResult<()> {
        // resolve the receiver value
        let receiver_value = if self.receiver_is_namespace_reference(receiver_id) {
            None
        } else {
            let (value, _) = self.lower_value_expression(receiver_id)?;
            Some(value)
        };

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = receiver_value {
            arguments.push(receiver);
        }
        arguments.push(value);

        // use interface dispatch metadata when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (self.dir_type_for_expression(receiver_id), receiver_value)
            && let Some((function_id, metadata)) = self.interface_dispatch_target(
                expression_id,
                receiver_type_id,
                receiver_value,
                target_symbol,
                self.env.type_lowerer.ty_void,
            )?
        {
            self.state
                .builder
                .call_void_with_metadata(function_id, arguments, metadata);
            return Ok(());
        }

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // emit the call
        self.state.builder.call_void(function_id, arguments);

        Ok(())
    }

    /// Lower an index expression to an element_get.
    pub(crate) fn lower_index_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        index_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the array value and index
        let (array_value, _array_type) = self.lower_value_expression(left_id)?;
        let (index_value, _index_type) = self.lower_value_expression(index_id)?;

        // get the result type (element type)
        let result_type = self.mir_type_for_expression(expression_id)?;

        // emit element_get
        let value = self.state.builder.element_get(array_value, index_value);
        Ok((value, result_type))
    }
}
