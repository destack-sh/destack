use cranelift_codegen::ir as cir;
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig, TargetIsa};
use destack_artifact::MirOptimized;
use destack_source::ModuleId;

use crate::EmitError;

/// Project MIR types into native calling and register representations.
pub(in crate::emit::native) struct TypeEmitter<'a> {
    /// Optimized MIR containing canonical physical layouts.
    pub(in crate::emit::native) optimized: &'a MirOptimized,
    /// Native target ISA.
    pub(in crate::emit::native) isa: &'a dyn TargetIsa,
    /// Module receiving diagnostics.
    module: ModuleId,
}

impl<'a> TypeEmitter<'a> {
    /// Create one native type emitter.
    pub(in crate::emit::native) const fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        isa: &'a dyn TargetIsa,
    ) -> Self {
        Self {
            optimized,
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

    /// Build one internal native type diagnostic.
    pub(in crate::emit::native) fn unsupported(&self, message: &str) -> EmitError {
        EmitError::Internal {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }
}
