use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the checked type table of one module.
    pub(in crate::lower) fn types(
        &self,
        module: ModuleId,
    ) -> CompilerResult<&dir::TypeTable<'static>> {
        Ok(&self.state(module)?.types)
    }

    /// Return one checked type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing type {:?}", ty.local_id),
            })
    }

    /// Return the checked reduction of one type id.
    pub(in crate::lower) fn reduced_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        Ok(self.types(ty.module_id)?.get_reduced_type_id(ty))
    }

    /// Return one canonical checked singleton in a well-known memory domain.
    pub(in crate::lower) fn memory_literal(
        &self,
        ty: dir::GlobalTypeId,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::MemoryLiteral> {
        let ty = self.reduced_type(ty)?;
        let actual = self.ty(ty)?;
        let dir::Type::Memory(literal) = actual else {
            return Err(CompilerError::Internal {
                message: format!("checked DIR left a {kind:?} singleton as {actual:?}"),
            });
        };
        if literal.kind_language_item() != kind.language_item() {
            return Err(CompilerError::Internal {
                message: format!("checked DIR supplied the wrong {kind:?} singleton"),
            });
        }

        Ok(literal)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
            other => {
                return Err(CompilerError::Internal {
                    message: format!("checked DIR declared a non-callable function: {other:?}"),
                });
            }
        };
        let dir::Type::FunctionSignature(signature) = signature else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a signature behind one function type".to_string(),
            });
        };

        Ok((signature, owner))
    }

}
