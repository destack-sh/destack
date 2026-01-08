use destack_dir::{Declaration, LocalNodeId};

use crate::{LowerError, LowerResult};

use super::super::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a declaration into MIR.
    pub(crate) fn lower_declaration(
        &mut self,
        declaration_id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<()> {
        match declaration {
            Declaration::Function { .. } => {
                let _ = self.lower_function(declaration_id, declaration)?;
                Ok(())
            }
            // struct declarations: pre-lower the struct type so it's cached for later use
            Declaration::Struct { descriptor, .. } => {
                let symbol = descriptor.symbol.into_global(self.module_id);
                // get the instance type for this struct symbol
                if let Some(instance_type_id) = self.types.get_instance_type_id(symbol) {
                    // lower the type to cache it in the type lowerer
                    self.type_lowerer.lower_type(
                        self.types,
                        instance_type_id,
                        self.module_id,
                        declaration_id.into_global_any(self.module_id),
                        &mut self.builder,
                    )?;
                }
                Ok(())
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: declaration_id.into_global_any(self.module_id),
                message: "unsupported declaration".to_string(),
            }),
        }
    }
}
