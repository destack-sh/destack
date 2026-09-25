use tspp_artifact::MirOptimized;
use tspp_bytecode as bytecode;
use tspp_mir as mir;
use tspp_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

/// Bytecode value representations and object-local type identities.
#[derive(Debug)]
pub(crate) struct TypeEmitter<'a> {
    /// Optimized MIR being emitted.
    pub(super) optimized: &'a MirOptimized,
    /// Module owning the emitted MIR.
    module: ModuleId,
    /// Common object identity assignments.
    object: &'a ObjectEmitter,
}

impl<'a> TypeEmitter<'a> {
    /// Create one bytecode type emitter over common object identities.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
    ) -> Self {
        Self {
            optimized,
            module,
            object,
        }
    }

    /// Return the object-local bytecode type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::TypeId) -> Result<bytecode::TypeId, EmitError> {
        let index = self
            .object
            .type_index(ty)
            .ok_or_else(|| self.missing("type"))?;

        Ok(bytecode::TypeId(index as u32))
    }

    /// Return the object-local bytecode function id for one MIR function.
    pub(crate) fn function_id(
        &self,
        function: mir::FunctionId,
    ) -> Result<bytecode::FunctionId, EmitError> {
        let index = self
            .object
            .function_index(function)
            .ok_or_else(|| self.missing("function"))?;

        Ok(bytecode::FunctionId(index as u32))
    }

    /// Return the object-local bytecode global id for one MIR global.
    pub(crate) fn global_id(&self, global: mir::GlobalId) -> Result<bytecode::GlobalId, EmitError> {
        let index = self
            .object
            .global_index(global)
            .ok_or_else(|| self.missing("global"))?;

        Ok(bytecode::GlobalId(index as u32))
    }

    /// Return the object-local dynamic table id for one implementation pair.
    pub(crate) fn dynamic_id(
        &self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> Result<u32, EmitError> {
        self.object
            .dynamic_index(concrete, constraint)
            .ok_or_else(|| self.missing("dynamic table"))
    }

    /// Build one missing common object item diagnostic.
    pub(super) fn missing(&self, item: &str) -> EmitError {
        ObjectEmitter::internal(
            self.module,
            &format!("{item} is absent from the common object"),
        )
    }

    /// Build one internal bytecode emission diagnostic.
    pub(super) fn internal(&self, message: &str) -> EmitError {
        ObjectEmitter::internal(self.module, message)
    }

    /// Build one unsupported type diagnostic.
    pub(super) fn unsupported_type(&self) -> EmitError {
        EmitError::UnsupportedType {
            anchor: self.module.into(),
            module: self.module,
        }
    }
}
