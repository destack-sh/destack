use destack_dir as dir;
use destack_dir::{Declaration, Member};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Predeclare interface method stubs for dispatch metadata.
    pub(crate) fn predeclare_interface_methods(&mut self) -> LowerResult<()> {
        // scan interface declarations for method stubs
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            // skip non interface declarations
            let Declaration::Interface {
                descriptor,
                members,
                ..
            } = declaration
            else {
                continue;
            };

            // resolve the interface symbol
            let interface_symbol = descriptor.symbol.into_global(self.module_id);

            // resolve the interface instance type
            let interface_type_id = self
                .types
                .get_instance_type_id(interface_symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "interface missing instance type".to_string(),
                })?;
            // capture a stable anchor for lowering
            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));

            // lower the interface mir type
            let interface_mir_type = self.type_lowerer.lower_type(
                self.types,
                interface_type_id,
                self.module_id,
                anchor,
                &mut self.builder,
            )?;

            // predeclare interface methods
            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                let Member::Method {
                    modifiers,
                    key,
                    signature,
                    symbol,
                    ..
                } = member
                else {
                    continue;
                };

                // skip non instance members
                if self.member_is_static(modifiers.as_ref())
                    || self.member_is_private(modifiers.as_ref())
                {
                    continue;
                }

                // skip already declared stubs
                let method_symbol = symbol.into_global(self.module_id);
                if self.functions_by_symbol.contains_key(&method_symbol) {
                    continue;
                }

                // resolve the method name
                let method_name =
                    self.member_dispatch_name_or_error(key.as_ref(), signature.mode, *member_id)?;
                let name_str = self.compiler.program.strings.get(method_name).to_string();

                // resolve signature types
                let return_type = self.resolve_method_return_type(
                    *member_id,
                    member_id.into_global_any(self.module_id),
                )?;
                let parameter_types =
                    self.method_parameter_types(signature, Some(interface_mir_type))?;

                // resolve the signature type for direct callsites
                let signature_type =
                    self.signature_mir_type_for_node(member_id.into_global_any(self.module_id))?;

                // declare the method stub
                let function_id =
                    self.builder
                        .extern_function(&name_str, &parameter_types, return_type);
                self.functions_by_symbol.insert(method_symbol, function_id);
                self.function_signature_types
                    .insert(function_id, signature_type);
            }
        }

        Ok(())
    }
}
