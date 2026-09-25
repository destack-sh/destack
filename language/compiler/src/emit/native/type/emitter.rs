use cranelift_codegen::ir as cir;
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig, TargetIsa};
use tspp_artifact::MirOptimized;
use tspp_mir as mir;
use tspp_mir::TargetLayout;
use tspp_source::ModuleId;

use crate::EmitError;

/// Project MIR types into native calling and register representations.
pub(in crate::emit::native) struct TypeEmitter<'a> {
    /// Optimized MIR containing canonical physical layouts.
    pub(in crate::emit::native) optimized: &'a MirOptimized,
    /// Target ABI layout.
    pub(in crate::emit::native) layout: TargetLayout,
    /// Native target ISA.
    pub(in crate::emit::native) isa: &'a dyn TargetIsa,
    /// Module receiving diagnostics.
    module: ModuleId,
}

impl<'a> TypeEmitter<'a> {
    /// Create one native type emitter.
    pub(in crate::emit::native) const fn new(
        module: ModuleId,
        layout: TargetLayout,
        optimized: &'a MirOptimized,
        isa: &'a dyn TargetIsa,
    ) -> Self {
        Self {
            optimized,
            layout,
            isa,
            module,
        }
    }

    /// Return the native pointer integer type.
    pub(in crate::emit::native) fn pointer(&self) -> cir::Type {
        self.isa.pointer_type()
    }

    /// Return the native calling convention.
    pub(in crate::emit::native) fn call_conv(&self) -> CallConv {
        self.isa.default_call_conv()
    }

    /// Return the native frontend memory configuration.
    pub(in crate::emit::native) fn frontend_config(&self) -> TargetFrontendConfig {
        self.isa.frontend_config()
    }

    /// Return whether one MIR type has a signed integer representation.
    pub(in crate::emit::native) fn is_signed_integer(
        &self,
        ty: mir::TypeId,
    ) -> Result<bool, EmitError> {
        // resolve the integer storage type
        let ty = self.optimized.tree.storage_type(ty);

        self.optimized
            .tree
            .type_definition(ty)
            .integer(self.layout.pointer_bits())
            .map(|(_, is_signed)| is_signed)
            .ok_or_else(|| self.unsupported("native operation requires an integer type"))
    }

    /// Return the largest mathematical value of one integer representation.
    pub(in crate::emit::native) const fn integer_maximum(
        &self,
        width: u16,
        is_signed: bool,
    ) -> u128 {
        if is_signed {
            (1u128 << (width - 1)) - 1
        } else if width == 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        }
    }

    /// Build one internal native type diagnostic.
    pub(in crate::emit::native) fn unsupported(&self, message: &str) -> EmitError {
        EmitError::Internal {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }
}
