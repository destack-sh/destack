use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use super::lower::TypeLowerer;
use crate::lower::{LifetimeParameters, ModuleLowerer, TypeSubstitution};
use crate::{CompilerError, CompilerResult, LowerError};

/// The parameter and result types lowered from one callable signature.
pub(in crate::lower) struct LoweredSignature {
    /// The parameter types in declaration order.
    pub(in crate::lower) parameters: Vec<mir::TypeId>,
    /// The callable result type.
    pub(in crate::lower) result: mir::TypeId,
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

impl TypeLowerer<'_, '_> {
    /// Lower one function value type to its two-word callable pair.
    pub(in crate::lower) fn lower_function(
        &mut self,
        function: &dir::FunctionType,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let signature = self.lower_signature_type(function.signature)?;
        let environment = self.lower_environment(function.environment)?;

        Ok(self.tree.intern_type(mir::Type::Function {
            signature: mir::TypeId::from(signature),
            environment: mir::TypeId::from(environment),
        }))
    }

    /// Lower one captured environment to its one-word reference.
    fn lower_environment(
        &mut self,
        environment: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let reduced = self.lowerer.reduced_type(environment)?;
        if !matches!(self.lowerer.ty(reduced)?, dir::Type::Unknown) {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a concrete function environment".to_string(),
            }
            .into());
        }

        Ok(self.erased_environment())
    }

    /// Return the erased nullable reference every plain function binds.
    pub(in crate::lower) fn erased_environment(&mut self) -> mir::LocalNodeId<mir::Type> {
        let row = self.tree.intern_type(mir::Type::Struct {
            fields: Vec::new(),
            copy: mir::Copy::No,
        });

        self.tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
            access: mir::Access::Mutable,
            pointee: row,
            nullability: mir::Nullability::Null,
        })
    }

    /// Lower one callable signature type closed over its own lifetimes.
    pub(in crate::lower) fn lower_signature_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let id = self.lowerer.reduced_type(id)?;
        let dir::Type::FunctionSignature(signature) = self.lowerer.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "a function value without a signature".to_string(),
            });
        };
        let signature = *self.lowerer.types(id.module_id)?.signature(signature);
        if signature.this_parameter.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method-typed function value".to_string(),
            }
            .into());
        }

        self.lower_signature_row(&signature, id.module_id)
    }

    /// Lower one signature row, leaving any receiver to its dispatch.
    pub(in crate::lower) fn lower_signature_row(
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
        let rows = self
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
        let mut parameters = Vec::with_capacity(rows.len());
        for row in rows {
            let ty = types.lower(row.ty)?;
            parameters.push(mir::SignatureParameter {
                ty: mir::TypeId::from(ty),
                obligations: Vec::new(),
            });
        }
        let result = match signature.return_type {
            Some(ty) => types.lower(ty)?,
            None => types.tree.intern_type(mir::Type::Void),
        };
        let lifetimes = lifetime_parameters.declarations(self.lowerer.strings);

        Ok(self.tree.intern_type(mir::Type::FunctionSignature {
            lifetimes,
            parameters,
            result: mir::TypeId::from(result),
        }))
    }
}
