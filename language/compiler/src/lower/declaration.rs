use destack_dir::{Declaration, LocalNodeId};

use crate::{LowerError, LowerResult};

use super::ModuleLowerer;

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
            _ => Err(LowerError::UnsupportedConstruct {
                node: declaration_id.into_global_any(self.module_id),
                message: "unsupported declaration".to_string(),
            }),
        }
    }
}
