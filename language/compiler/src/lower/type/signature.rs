use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{GenericScope, ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one callable signature into MIR parameter and result types.
    fn signature_types(
        &mut self,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<(Vec<mir::TypeId>, mir::TypeId)> {
        let (signature, owner) = self.lower.signature(declared)?;
        let signature = *self.lower.types(owner)?.signature(signature);
        let parameter_types = self
            .lower
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

        Ok((parameters, result))
    }

    /// Lower one callable signature into a MIR signature type.
    pub(in crate::lower) fn lower_callable_signature(
        &mut self,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        let signature = self.lower_signature_type(declared)?;

        Ok(signature)
    }
}

impl ModuleLowerer<'_> {
    /// Lower one callable signature to its parameter and result types.
    pub(in crate::lower) fn lower_signature(
        &mut self,
        tree: &mut mir::Tree,
        declared: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<(Vec<mir::TypeId>, mir::TypeId)> {
        let mut lower = self.type_lowerer(tree, scope);

        lower.signature_types(declared)
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one callable signature type closed over its own lifetimes.
    fn lower_signature_type(&mut self, id: dir::GlobalTypeId) -> CompilerResult<mir::TypeId> {
        // require a plain function signature without a receiver
        let (_, owner) = self.lower.signature(id)?;
        let dir::Type::FunctionSignature(signature) = self.lower.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "a function value without a signature".to_string(),
            });
        };
        let signature = *self.lower.types(owner)?.signature(signature);
        if signature.this_parameter.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a method-typed function value".to_string(),
            }
            .into());
        }

        self.lower_bare_signature(&signature, owner, None, None)
    }

    /// Lower one signature, leaving any receiver to its dispatch.
    pub(in crate::lower) fn lower_bare_signature(
        &mut self,
        signature: &dir::FunctionSignatureType,
        module: ModuleId,
        template: Option<dir::GlobalGenericTemplateId>,
        declaration: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<mir::TypeId> {
        // close the signature over its own lifetime slots beneath the enclosing binder
        let template = self
            .lower
            .signature_template(signature.template.or(template), Some(self.scope))?;
        let scope = GenericScope::for_signature(self.lower, template, declaration)?
            .nested_in(self.scope, self.lower, self.tree)?;

        // read the parameters the signature declares
        let declared = self
            .lower
            .types(module)?
            .parameters(signature.parameters)
            .to_vec();

        // lower the parameters under the signature scope, keeping the interface's own this
        let mut types = self.lower.type_lowerer(self.tree, &scope);
        types.this_type = self.this_type;
        let mut parameters = Vec::with_capacity(declared.len());
        for parameter in declared {
            let ty = types.lower(parameter.ty)?;
            parameters.push(mir::SignatureParameter::new(ty));
        }

        // lower the result and declare the collected lifetime slots
        let result = match signature.return_type {
            Some(ty) => types.lower(ty)?,
            None => types.tree.void_type(),
        };
        let lifetimes = scope.declarations(self.lower.strings);

        Ok(self.tree.intern_type(mir::Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        }))
    }
}
