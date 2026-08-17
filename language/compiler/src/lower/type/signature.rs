use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{LifetimeParameters, ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one checked callable signature into MIR parameter and result types.
    fn lower_signature(
        &mut self,
        declared: dir::GlobalTypeId,
        widens_optional: bool,
    ) -> CompilerResult<(Vec<mir::TypeId>, mir::TypeId)> {
        // resolve the written signature through its materialized types
        let declared = self.lowerer.instance_type(self.instance, declared)?;
        let (signature, owner) = self.lowerer.signature(declared)?;
        let signature = self.lowerer.types(owner)?.signature(signature);
        let parameter_types = self
            .lowerer
            .types(owner)?
            .parameters(signature.parameters)
            .iter()
            .map(|parameter| (parameter.ty, parameter.is_optional))
            .collect::<Vec<_>>();
        let return_type = signature.return_type;

        // lower parameters in their declared order
        let mut parameters = Vec::with_capacity(parameter_types.len());
        for (ty, is_optional) in parameter_types {
            let mut parameter = self.lower(ty)?;

            // widen defaulted parameters into their undefined carrier
            if widens_optional && self.widens_optional_parameter(ty, is_optional)? {
                parameter = self.insert_optional_carrier(parameter)?;
            }
            parameters.push(parameter);
        }

        // lower the result, using void for an omitted return annotation
        let result = match return_type {
            Some(ty) => self.lower(ty)?,
            None => self.tree.void_type(),
        };

        Ok((parameters, result))
    }

    /// Return whether one defaulted parameter widens into its undefined carrier.
    fn widens_optional_parameter(
        &mut self,
        ty: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<bool> {
        if !is_optional {
            return Ok(false);
        }
        let grounded = self.lowerer.instance_type(self.instance, ty)?;

        Ok(!self.lowerer.contains_undefined(grounded)?)
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
    /// Return whether one type carries an undefined member.
    fn contains_undefined(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        match self.ty(ty)? {
            dir::Type::Undefined => Ok(true),
            dir::Type::Union(union) => {
                for member in self.types(ty.module_id)?.type_ids(union.elements).to_vec() {
                    if matches!(self.ty(member)?, dir::Type::Undefined) {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Lower one callable signature to its parameter and result types.
    pub(in crate::lower) fn lower_signature(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<(Vec<mir::TypeId>, mir::TypeId)> {
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters)
            .with_instance(specialization);

        lowerer.lower_signature(declared, true)
    }

    /// Lower one binding signature at the host's exact calling convention.
    pub(in crate::lower) fn lower_host_signature(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<(Vec<mir::TypeId>, mir::TypeId)> {
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer = self.type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters);

        lowerer.lower_signature(declared, false)
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

        // judge omission carriers before borrowing the parameter scope
        let mut widened = Vec::with_capacity(declared.len());
        for parameter in &declared {
            widened.push(self.widens_optional_parameter(parameter.ty, parameter.is_optional)?);
        }

        // lower the parameters and result under the signature scope
        let mut types = self
            .lowerer
            .type_lowerer(self.tree, self.pointer_bytes, &lifetime_parameters)
            .with_instance(self.instance);
        let mut parameters = Vec::with_capacity(declared.len());
        for (parameter, widen) in declared.into_iter().zip(widened) {
            let mut ty = types.lower(parameter.ty)?;

            // widen defaulted parameters so omitted calls pass the undefined case
            if widen {
                ty = types.insert_optional_carrier(ty)?;
            }
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
