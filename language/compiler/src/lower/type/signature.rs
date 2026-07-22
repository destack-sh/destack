use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{LifetimeParameters, ModuleLowerer, TypeSubstitution};

/// MIR parameter and result types lowered from one checked callable signature.
pub(in crate::lower) struct LoweredSignature {
    /// The parameter types in declaration order.
    pub(in crate::lower) parameters: Vec<mir::TypeId>,
    /// The callable result type.
    pub(in crate::lower) result: mir::TypeId,
}

impl ModuleLowerer<'_> {
    /// Lower one checked callable signature to MIR types.
    pub(in crate::lower) fn lower_signature(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<LoweredSignature> {
        let (signature, owner) = self.signature(declared)?;
        let signature = self.types(owner)?.signature(signature);
        let parameter_list = signature.parameters;
        let parameter_types: Vec<_> = self
            .types(owner)?
            .parameters(parameter_list)
            .iter()
            .map(|parameter| parameter.ty)
            .collect();
        let return_type = signature.return_type;

        // lower every parameter under the same generic environment
        let pointer_bytes = builder.pointer_bytes();
        let mut parameters = Vec::with_capacity(parameter_types.len());
        for ty in parameter_types {
            let ty = self
                .type_lowerer(
                    builder.tree_mut(),
                    pointer_bytes,
                    type_substitution,
                    lifetime_parameters,
                )
                .lower(ty)?;
            parameters.push(ty);
        }

        // lower the result, using void for an omitted return annotation
        let result = match return_type {
            Some(ty) => self
                .type_lowerer(
                    builder.tree_mut(),
                    pointer_bytes,
                    type_substitution,
                    lifetime_parameters,
                )
                .lower(ty)?,
            None => builder.tree_mut().intern_type(mir::Type::Void),
        };

        Ok(LoweredSignature { parameters, result })
    }
}
