use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare an import header for every collected foreign call demand.
    pub(in crate::lower) fn declare_imported_functions(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        imports: FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        // declare each import from its checked signature under its canonical name
        for symbol in imports {
            if self.functions.contains_key(&(symbol, Vec::new())) {
                continue;
            }
            let Some(state) = self.modules.get(&symbol.module_id) else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "checked DIR referenced the unloaded module {:?}",
                        symbol.module_id
                    ),
                });
            };
            let Some(name) = state.bindings.get_symbol(symbol.local_id).name() else {
                return Err(CompilerError::Internal {
                    message: "checked DIR imported a function without a name".to_string(),
                });
            };
            let name = format!("{}.{}", state.path, self.strings.get(name));

            // resolve the imported signature through the owning module
            let declared = self.symbol_type(symbol)?;
            let lifetimes = self.signature_lifetimes(declared)?;
            self.lifetime_slots = lifetimes.clone();
            let (signature, owner) = match self.ty(declared)? {
                dir::Type::Function(function) => match self.ty(function.signature)? {
                    dir::Type::FunctionSignature(signature) => {
                        (signature, function.signature.module_id)
                    }
                    _ => {
                        return Err(CompilerError::Internal {
                            message: "checked DIR imported a callable without a signature"
                                .to_string(),
                        });
                    }
                },
                dir::Type::FunctionSignature(signature) => (signature, declared.module_id),
                _ => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR imported a non-function callable".to_string(),
                    });
                }
            };
            let row = self.types(owner)?.signature(signature);
            let (parameter_list, return_type) = (row.parameters, row.return_type);
            let count = self.types(owner)?.parameters(parameter_list).len();
            let mut parameters = Vec::with_capacity(count);
            for index in 0..count {
                let ty = self.types(owner)?.parameters(parameter_list)[index].ty;
                parameters.push(self.lower_type_id(builder.tree_mut(), ty)?);
            }
            let result = match return_type {
                Some(return_type) => self.lower_type_id(builder.tree_mut(), return_type)?,
                None => builder.tree_mut().insert(mir::Type::Void),
            };

            let mut header = builder.function_header(&name);
            for slot in 0..lifetimes.len() {
                header = header.lifetime(&format!("L{slot}"));
            }
            let header = header.parameters(parameters).result(result);
            let function = builder.external_function(header);
            self.functions.insert((symbol, Vec::new()), function);
        }

        Ok(())
    }
}
