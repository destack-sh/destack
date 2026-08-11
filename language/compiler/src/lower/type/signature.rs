use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{LifetimeParameters, ModuleLowerer, TypeLowerer, TypeSubstitution};
use crate::{CompilerError, CompilerResult, LowerError};

/// The parameter and result types lowered from one callable signature.
pub(in crate::lower) struct LoweredSignature {
    /// The parameter types in declaration order.
    pub(in crate::lower) parameters: Vec<mir::TypeId>,
    /// The callable result type.
    pub(in crate::lower) result: mir::TypeId,
}

impl TypeLowerer<'_, '_> {
    /// Lower one checked callable signature into MIR parameter and result types.
    fn lower_signature(&mut self, declared: dir::GlobalTypeId) -> CompilerResult<LoweredSignature> {
        let (signature, owner) = self.lowerer.signature(declared)?;
        let signature = self.lowerer.types(owner)?.signature(signature);
        let parameter_types = self
            .lowerer
            .types(owner)?
            .parameters(signature.parameters)
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();
        let return_type = signature.return_type;

        // lower parameters in their declared order
        let mut parameters = Vec::with_capacity(parameter_types.len());
        for ty in parameter_types {
            parameters.push(self.lower(ty)?);
        }

        // lower the result, using void for an omitted return annotation
        let result = match return_type {
            Some(ty) => self.lower(ty)?,
            None => self.tree.void_type(),
        };

        Ok(LoweredSignature { parameters, result })
    }

    /// Lower one checked callable signature into a MIR signature type.
    pub(in crate::lower) fn lower_callable_signature(
        &mut self,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        let signature = self.lower_signature_type(declared)?;

        Ok(mir::TypeId::from(signature))
    }
}

impl ModuleLowerer<'_> {
    /// Lower one callable signature to its parameter and result types.
    pub(in crate::lower) fn lower_signature(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
        type_substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<LoweredSignature> {
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer = self.type_lowerer(
            builder.tree_mut(),
            pointer_bytes,
            type_substitution,
            lifetime_parameters,
        );

        lowerer.lower_signature(declared)
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one callable signature type closed over its own lifetimes.
    fn lower_signature_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let (_, owner) = self.lowerer.signature(id)?;
        let dir::Type::FunctionSignature(signature) = self.lowerer.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "a function value without a signature".to_string(),
            });
        };
        let signature = *self.lowerer.types(owner)?.signature(signature);
        if signature.this_parameter.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method-typed function value".to_string(),
            }
            .into());
        }

        self.lower_bare_signature(&signature, owner)
    }

    /// Lower one signature, leaving any receiver to its dispatch.
    pub(in crate::lower) fn lower_bare_signature(
        &mut self,
        signature: &dir::FunctionSignatureType,
        module: ModuleId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        if signature.asynchrony != dir::Asynchrony::Sync || signature.is_generator {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an asynchronous function value".to_string(),
            }
            .into());
        }

        // close the signature over its own lifetime slots
        let lifetime_parameters = match signature.template {
            Some(template) => LifetimeParameters::from_template(self.lowerer, template)?,
            None => LifetimeParameters::default(),
        };
        let declared = self
            .lowerer
            .types(module)?
            .parameters(signature.parameters)
            .to_vec();

        // lower the parameters and result under the signature scope
        let mut types = self.lowerer.type_lowerer(
            self.tree,
            self.pointer_bytes,
            self.type_substitution,
            &lifetime_parameters,
        );
        let mut parameters = Vec::with_capacity(declared.len());
        for parameter in declared {
            let ty = types.lower(parameter.ty)?;
            parameters.push(mir::SignatureParameter::new(ty));
        }
        let result = match signature.return_type {
            Some(ty) => types.lower(ty)?,
            None => types.tree.void_type(),
        };
        let lifetimes = lifetime_parameters.declarations(self.lowerer.strings);

        Ok(self.tree.intern_type(mir::Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        }))
    }
}
