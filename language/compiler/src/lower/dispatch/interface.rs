use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Declare a stub function for an interface method.
    pub(crate) fn lower_interface_method_stub(
        &mut self,
        interface_symbol: dir::GlobalSymbolId,
        interface_type: mir::LocalNodeId<mir::Type>,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        key: Option<&dir::Key>,
        signature: &dir::FunctionSignature,
        method_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        if let Some(function_id) = self.function_for_symbol(method_symbol) {
            return Ok(function_id);
        }

        let method_name =
            self.member_dispatch_name_or_error(key, signature.mode, member_id.into_any())?;
        let method_name = self.strings.get(method_name).to_string();
        let name_str = if let Some(owner_name) = self.symbol_path_name(interface_symbol) {
            format!("{owner_name}.{method_name}")
        } else {
            method_name
        };

        // resolve return type for the interface method
        let signature_type_id =
            self.signature_type_id_for_node(member_id.into_global_any(self.module_id))?;
        let dir::Type::Function { return_type, .. } = self.types.get_type(signature_type_id) else {
            return Err(self.missing_type_error(member_id.into_global_any(self.module_id)));
        };
        let return_type = if let Some(return_type_id) = return_type {
            self.lower_type(
                *return_type_id,
                member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
            )?
        } else {
            self.type_lowerer.ty_void
        };

        // resolve parameter types, including the interface receiver
        let parameter_types = self.method_parameter_types(signature, Some(interface_type))?;

        // build a MIR signature type aligned with the lowered parameters
        let signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        self.builder.type_function_pointer(signature);

        // declare the interface method stub
        let function_id = self
            .builder
            .extern_function(&name_str, &parameter_types, return_type);
        // register function bindings
        self.register_function_binding_for_symbol(method_symbol, function_id, signature)?;

        // ensure the interface instance type is registered
        Ok(function_id)
    }
}
