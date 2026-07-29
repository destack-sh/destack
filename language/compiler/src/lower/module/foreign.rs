use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    CallableImplementation, GenericInstanceKey, LifetimeParameters, ModuleLowerer, ReceiverBinding,
    TypeSubstitution,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// An imported member callable's owner and declared role.
struct ImportedMember {
    /// The owning nominal symbol.
    owner: dir::GlobalSymbolId,
    /// The declared member role.
    role: Option<dir::FunctionRole>,
}

impl ModuleLowerer<'_> {
    /// Declare an import header for every referenced foreign callable.
    pub(in crate::lower) fn declare_imported_functions(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        imports: FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        // declare each import from its checked signature under its canonical name
        for symbol in imports {
            let key = GenericInstanceKey::non_generic(symbol);
            if self.functions.contains_key(&key) {
                continue;
            }
            let path = self.state(symbol.module_id)?.path.clone();

            // resolve the imported signature through the owning module
            let declared = self.symbol_type(symbol)?;
            let lifetime_parameters = self.lifetime_parameters(declared)?;
            let (signature, owner) = self.signature(declared)?;

            // member callables lead with their receiver and qualify by owner
            let member = self.imported_member(symbol)?;
            let type_substitution = match &member {
                Some(member) => TypeSubstitution::default().with_receiver(
                    ReceiverBinding::Application(dir::GenericApplication {
                        symbol: member.owner,
                        arguments: dir::TypeListId::EMPTY,
                    }),
                ),
                None => TypeSubstitution::default(),
            };
            let (name, receiver) = match &member {
                Some(member) => {
                    let (name, receiver) = self.imported_member_header(
                        builder,
                        symbol,
                        member,
                        signature,
                        owner,
                        &type_substitution,
                        &lifetime_parameters,
                    )?;

                    (format!("{path}.{name}"), Some(receiver))
                }
                None => {
                    let Some(name) = self.symbol_name(symbol)? else {
                        return Err(CompilerError::Internal {
                            message: "checked DIR imported a function without a name".to_string(),
                        });
                    };

                    (format!("{path}.{}", self.strings.get(name)), None)
                }
            };

            let signature =
                self.lower_signature(builder, declared, &type_substitution, &lifetime_parameters)?;
            let mut parameters = signature.parameters;
            if let Some(receiver) = receiver {
                parameters.insert(0, receiver);
            }

            // constructors initialize storage and return no value
            let is_constructor = member
                .as_ref()
                .is_some_and(|member| member.role == Some(dir::FunctionRole::Constructor));
            let result = match is_constructor {
                true => builder.tree_mut().intern_type(mir::Type::Void),
                false => signature.result,
            };

            let header = lifetime_parameters.declare(builder.function_header(&name));
            let header = header.parameters(parameters).result(result);
            let function = builder.external_function(header);
            self.functions.insert(key, function);
        }

        Ok(())
    }

    /// Declare the dotted-name extern behind one sealed binding.
    pub(in crate::lower) fn declare_binding_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(());
        }
        let Some(CallableImplementation::Binding { name: Some(name) }) =
            self.callable_implementation(symbol)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an unnamed binding callable".to_string(),
            }
            .into());
        };

        // the declared signature supplies the parameter and return carriers
        let declared = self.symbol_type(symbol)?;
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let signature =
            self.lower_signature(builder, declared, &type_substitution, &lifetime_parameters)?;

        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.binding_function(header, &name);

        // bindings observe external state until the sealed effect row refines them
        *builder.effects_mut().function_mut(function) = mir::FunctionEffect::unknown();
        self.functions.insert(key, function);

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
        type_substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<(String, mir::LocalNodeId<mir::Type>)> {
        let pointer_bytes = builder.pointer_bytes();
        let value = self
            .type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                type_substitution,
                lifetime_parameters,
            )
            .lower_nominal(member.owner, &[])?;
        let pointee = value.storage;
        let receiver = match member.role {
            // constructors initialize storage exclusively
            Some(dir::FunctionRole::Constructor) => {
                builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime: mir::Lifetime::empty(),
                    space: mir::Space::Local,
                    access: mir::Access::Exclusive,
                    pointee,
                    nullability: mir::Nullability::None,
                })
            }
            // methods receive this at their sealed receiver type
            _ => {
                let sealed = self.types(owner)?.signature(signature).this_parameter;
                let Some(sealed) = sealed else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR imported an instance method without a receiver"
                            .to_string(),
                    });
                };

                self.type_lowerer(
                    builder.tree_mut(),
                    pointer_bytes,
                    type_substitution,
                    lifetime_parameters,
                )
                .lower(sealed)?
            }
        };

        let Some(owner_name) = self.symbol_name(member.owner)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR imported a method from an unnamed nominal".to_string(),
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
        let dir::Member::Method { signature, .. } = state.tree().get(member) else {
            return Ok(None);
        };
        let role = signature.role;

        // the introducing scope's owner names the enclosing nominal
        let Some(owner) = state.bindings.get_scope(declared.scope).owner else {
            return Err(CompilerError::Internal {
                message: "checked DIR imported a member outside a nominal scope".to_string(),
            });
        };

        Ok(Some(ImportedMember {
            owner: owner.into_global(symbol.module_id),
            role,
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
                message: "checked DIR imported a method without a name".to_string(),
            }),
        }
    }
}
