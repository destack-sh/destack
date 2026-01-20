use destack_dir::{DynamicKey, GlobalSymbolId, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Declare a stub function for an interface method.
    pub(crate) fn lower_interface_method_stub(
        &mut self,
        interface_type: mir::LocalNodeId<mir::Type>,
        member_id: LocalNodeId<Member>,
        key: Option<&DynamicKey>,
        signature: &dir::FunctionSignature,
        method_symbol: GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        if let Some(function_id) = self.functions_by_symbol.get(&method_symbol).copied() {
            return Ok(function_id);
        }

        let method_name = self.member_dispatch_name_or_error(key, signature.mode, member_id)?;
        let name_str = self.compiler.program.strings.get(method_name).to_string();

        // resolve return type for the interface method
        let return_type =
            self.resolve_method_return_type(member_id, member_id.into_global_any(self.module_id))?;

        // resolve parameter types, including the interface receiver
        let parameter_types = self.method_parameter_types(signature, Some(interface_type))?;

        // resolve the signature type for direct callsites
        let signature_type =
            self.signature_mir_type_for_node(member_id.into_global_any(self.module_id))?;

        // declare the interface method stub
        let function_id = self
            .builder
            .extern_function(&name_str, &parameter_types, return_type);
        // register function bindings
        self.register_function_binding_for_symbol(method_symbol, function_id, signature_type)?;

        // ensure the interface instance type is registered
        Ok(function_id)
    }
}
