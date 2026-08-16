use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    CallableImplementation, FunctionDeclaration, GenericInstanceKey, LifetimeParameters,
    ModuleLowerer,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// An imported member callable's owner and declared role.
struct ImportedMember {
    /// The owning nominal symbol.
    owner: dir::GlobalSymbolId,
    /// The declared member role.
    role: Option<dir::FunctionRole>,
    /// Whether the member declares no receiver.
    is_static: bool,
}

impl ModuleLowerer<'_> {
    /// Declare an import header for one referenced foreign callable.
    pub(in crate::lower) fn declare_imported_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // declare the import once under its canonical name
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(());
        }
        let path = self.state(symbol.module_id)?.path.clone();

        // resolve the imported signature through the owning module
        let declared = self.symbol_type(symbol)?;
        if self.signature_has_instance_parameters(declared)? {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an imported generic callable without instance arguments".to_string(),
            }
            .into());
        }
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let (signature, owner) = self.signature(declared)?;

        // lead member callables with their receiver and qualify by owner
        let member = self.imported_member(symbol)?;
        let (name, receiver) = match &member {
            Some(member) => {
                let (name, receiver) = self.imported_member_header(
                    builder,
                    symbol,
                    member,
                    signature,
                    owner,
                    &lifetime_parameters,
                )?;

                (format!("{path}.{name}"), receiver)
            }
            None => {
                let Some(name) = self.symbol_name(symbol)? else {
                    return Err(CompilerError::Internal {
                        message: "an imported function without a name".to_string(),
                    });
                };

                (format!("{path}.{}", self.strings.get(name)), None)
            }
        };

        // lower the signature, prepending the receiver of a method import
        let signature = self.lower_signature(builder, declared, None, &lifetime_parameters)?;
        let mut parameters = signature.parameters;
        if let Some(receiver) = receiver {
            parameters.insert(0, receiver);
        }

        // give constructors a void result
        let is_constructor = member
            .as_ref()
            .is_some_and(|member| member.role == Some(dir::FunctionRole::Constructor));
        let result = match is_constructor {
            true => builder.tree_mut().intern_type(mir::Type::Void),
            false => signature.result,
        };

        // declare the header as an external function under the imported name
        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header.parameters(parameters).result(result);
        let function = builder.external_function(header);
        self.functions
            .insert(key, FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Declare the imported global behind one foreign module constant.
    pub(in crate::lower) fn declare_imported_constant(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.globals.contains_key(&symbol) {
            return Ok(());
        }

        // declare the import at the constant's lowered type and dotted name
        let pointer_bytes = builder.pointer_bytes();
        let lifetimes = LifetimeParameters::default();
        let declared = self.symbol_type(symbol)?;
        let ty = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetimes)
            .lower(declared)?;
        let name = self.constant_name(symbol)?;
        let global = builder.external_global(&name, ty, mir::Mutability::Immutable);
        self.globals.insert(symbol, Ok(global));

        Ok(())
    }

    /// Declare the dotted-name extern behind one binding.
    pub(in crate::lower) fn declare_binding_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(());
        }
        let Some(CallableImplementation::Binding { binding }) =
            self.callable_implementation(symbol)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an unnamed binding callable".to_string(),
            }
            .into());
        };

        // read the parameter and return carriers from the declared signature
        let declared = self.symbol_type(symbol)?;

        // require a concrete signature for one extern header
        if self.signature_has_instance_parameters(declared)? {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic binding callable".to_string(),
            }
            .into());
        }

        // lower the signature outside any instance bindings
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let signature = self.lower_signature(builder, declared, None, &lifetime_parameters)?;

        // declare the header as a host binding under the dotted extern name
        let name = self.strings.get(binding.name);
        let header = lifetime_parameters.declare(builder.function_header(name));
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.binding_function(header, binding);

        // mark bindings as observing external state
        *builder.effects_mut().function_mut(function) = mir::FunctionEffect::unknown();
        self.functions
            .insert(key, FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Return one imported member's qualified name and receiver type.
    fn imported_member_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        member: &ImportedMember,
        signature: dir::FunctionSignatureId,
        owner: ModuleId,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<(String, Option<mir::LocalNodeId<mir::Type>>)> {
        let pointer_bytes = builder.pointer_bytes();
        let value = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters)
            .lower_nominal(member.owner, &[])?;
        let pointee = value.storage;
        let receiver = match member.role {
            // pass an exclusive reference to constructors
            Some(dir::FunctionRole::Constructor) => {
                Some(builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime: mir::Lifetime::empty(),
                    storage: mir::Storage::Heap(mir::Space::Local),
                    access: mir::Access::Exclusive,
                    pointee,
                    nullability: mir::Nullability::None,
                }))
            }
            // statics call without a receiver slot
            _ if member.is_static => None,
            // pass this at the declared receiver type
            _ => {
                let declared = self.types(owner)?.signature(signature).this_parameter;
                let Some(declared) = declared else {
                    return Err(CompilerError::Internal {
                        message: "an imported instance method without a receiver".to_string(),
                    });
                };

                Some(
                    self.type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters)
                        .lower(declared)?,
                )
            }
        };

        // name the extern after its owner and member
        let Some(owner_name) = self.symbol_name(member.owner)? else {
            return Err(CompilerError::Internal {
                message: "an imported method from an unnamed nominal".to_string(),
            });
        };
        let owner_name = self.strings.get(owner_name).to_string();
        let member_name = self.imported_member_name(symbol, member.role)?;

        Ok((format!("{owner_name}.{member_name}"), receiver))
    }

    /// Resolve one imported symbol's member declaration, when it names one.
    fn imported_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ImportedMember>> {
        let state = self.state(symbol.module_id)?;
        let declared = state.bindings.get_symbol(symbol.local_id);
        let Some(node) = declared.declaration else {
            return Ok(None);
        };
        let Ok(member) = node.local_id.try_into_typed::<dir::Member>() else {
            return Ok(None);
        };
        let dir::Member::Method {
            signature,
            is_static,
            ..
        } = state.tree().get(member)
        else {
            return Ok(None);
        };
        let role = signature.role;
        let is_static = *is_static;

        // read the enclosing nominal from the introducing scope's owner
        let Some(owner) = state.bindings.get_scope(declared.scope).owner else {
            return Err(CompilerError::Internal {
                message: "an imported member outside a nominal scope".to_string(),
            });
        };

        Ok(Some(ImportedMember {
            owner: owner.into_global(symbol.module_id),
            role,
            is_static,
        }))
    }

    /// Return one imported member's declared name.
    fn imported_member_name(
        &self,
        symbol: dir::GlobalSymbolId,
        role: Option<dir::FunctionRole>,
    ) -> CompilerResult<String> {
        match self.symbol_name(symbol)? {
            Some(name) => Ok(self.strings.get(name).to_string()),
            None if role == Some(dir::FunctionRole::Constructor) => Ok("constructor".to_string()),
            None => Err(CompilerError::Internal {
                message: "an imported method without a name".to_string(),
            }),
        }
    }
}
