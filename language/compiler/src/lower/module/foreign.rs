use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{AmbientCallable, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// An imported member callable's owner and declared role.
struct ImportedMember {
    /// The owning nominal symbol.
    owner: dir::GlobalSymbolId,
    /// The declared member role.
    role: Option<dir::FunctionRole>,
}

impl ModuleLowerer<'_> {
    /// Declare an import header for every collected foreign call demand.
    pub(in crate::lower) fn declare_imported_functions(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        imports: FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        // declare each import from its checked signature under its canonical name
        for symbol in imports {
            if self.functions.contains_key(&(symbol, Vec::new())) {
                continue;
            }
            let path = self.state(symbol.module_id)?.path.clone();

            // resolve the imported signature through the owning module
            let declared = self.symbol_type(symbol)?;
            let lifetimes = self.signature_lifetimes(declared)?;
            self.lifetime_slots = lifetimes.clone();
            let (signature, owner) = self.signature_of(declared)?;

            // member callables lead with their receiver and qualify by owner
            let member = self.imported_member(symbol)?;
            let (name, receiver) = match &member {
                Some(member) => {
                    let (name, receiver) =
                        self.imported_member_header(builder, symbol, member, signature, owner)?;

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

            let mut parameters = self.signature_parameter_carriers(builder, signature, owner)?;
            if let Some(receiver) = receiver {
                parameters.insert(0, receiver);
            }

            // constructors initialize storage and return no value
            let is_constructor = member
                .as_ref()
                .is_some_and(|member| member.role == Some(dir::FunctionRole::Constructor));
            let result = match is_constructor {
                true => builder.tree_mut().insert(mir::Type::Void),
                false => self.signature_result_carrier(builder, declared)?,
            };

            let mut header = builder.function_header(&name);
            for slot in 0..lifetimes.len() {
                header = header.lifetime(&format!("L{slot}"));
            }
            let header = header.parameters(parameters).result(result);
            let function = builder.external_function(header);
            self.functions.insert((symbol, Vec::new()), function);
        }

        Ok(())
    }

    /// Declare the dotted-name extern behind one sealed binding.
    pub(in crate::lower) fn declare_binding_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.functions.contains_key(&(symbol, Vec::new())) {
            return Ok(());
        }
        let Some(AmbientCallable::Binding { name: Some(name) }) = self.ambient_callable(symbol)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an unnamed binding callable".to_string(),
            }
            .into());
        };

        // the declared signature supplies the parameter and return carriers
        let declared = self.symbol_type(symbol)?;
        let (signature, owner) = self.signature_of(declared)?;
        let parameters = self.signature_parameter_carriers(builder, signature, owner)?;
        let result = self.signature_result_carrier(builder, declared)?;

        let header = builder
            .function_header(&name)
            .parameters(parameters)
            .result(result);
        let function = builder.binding_function(header, &name);

        // bindings observe external state until the sealed effect row refines them
        *builder.effects_mut().function_mut(function) = mir::FunctionEffect::unknown();
        self.functions.insert((symbol, Vec::new()), function);

        Ok(())
    }

    /// Lower one sealed signature's parameter carriers.
    fn signature_parameter_carriers(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        signature: dir::FunctionSignatureId,
        owner: ModuleId,
    ) -> CompilerResult<Vec<mir::LocalNodeId<mir::Type>>> {
        let row = self.types(owner)?.signature(signature);
        let parameter_list = row.parameters;
        let count = self.types(owner)?.parameters(parameter_list).len();
        let mut parameters = Vec::with_capacity(count + 1);
        for index in 0..count {
            let ty = self.types(owner)?.parameters(parameter_list)[index].ty;
            parameters.push(self.lower_type_id(builder.tree_mut(), ty)?);
        }

        Ok(parameters)
    }

    /// Lower one sealed signature's return carrier.
    fn signature_result_carrier(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.signature_return(declared)? {
            Some(return_type) => self.lower_type_id(builder.tree_mut(), return_type),
            None => Ok(builder.tree_mut().insert(mir::Type::Void)),
        }
    }

    /// Return one imported member's qualified name and receiver carrier.
    fn imported_member_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        member: &ImportedMember,
        signature: dir::FunctionSignatureId,
        owner: ModuleId,
    ) -> CompilerResult<(String, mir::LocalNodeId<mir::Type>)> {
        let value = self.ensure_nominal(builder.tree_mut(), member.owner)?;
        let pointee = self.nominals[&member.owner].ty;
        let receiver = match member.role {
            // constructors initialize storage exclusively
            Some(dir::FunctionRole::Constructor) => {
                builder.tree_mut().insert(mir::Type::Reference {
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

                self.lower_receiver(builder, sealed, pointee, value)?
            }
        };

        let Some(class_name) = self.symbol_name(member.owner)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR imported a method from an unnamed nominal".to_string(),
            });
        };
        let class_name = self.strings.get(class_name).to_string();
        let member_name = self.imported_member_name(symbol, member.role)?;

        Ok((format!("{class_name}.{member_name}"), receiver))
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
