use destack_dir::{Declaration, DeclarationDescriptor, LocalNodeId};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Get the descriptor for a nominal declaration.
    pub(crate) fn descriptor_for_declaration_or_error<'a>(
        &self,
        declaration_id: LocalNodeId<Declaration>,
        declaration: &'a Declaration,
    ) -> LowerResult<&'a DeclarationDescriptor> {
        // require a nominal declaration for constructor lowering
        let is_nominal = matches!(
            declaration,
            Declaration::Struct { .. } | Declaration::Class { .. }
        );
        if !is_nominal {
            return Err(LowerError::UnsupportedConstruct {
                node: declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "constructor must belong to a nominal declaration".to_string(),
            });
        }

        // return the declaration descriptor
        Ok(declaration.descriptor())
    }
}
