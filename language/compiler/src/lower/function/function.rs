use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{Body, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the MIR header for one function declaration with a body.
    pub(in crate::lower) fn declare_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Body> {
        let module = self.module;

        // declare the signature's lifetime generics before its parameter types
        let node = declaration.into_global_any(module);
        let Some(symbol) = self.symbol_declared_at(node)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one function declaration".to_string(),
            });
        };
        let lifetimes = self.signature_lifetimes(self.symbol_type(symbol)?)?;
        self.lifetime_slots = lifetimes.clone();

        // resolve the checked parameter types alongside their symbols
        let parameter_nodes = {
            let dir::Declaration::Function(function) = self.local().tree().get(declaration) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a function body outside a function".to_string(),
                });
            };

            function.signature.parameters.clone()
        };
        let mut parameters = Vec::with_capacity(parameter_nodes.len());
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in &parameter_nodes {
            let node = parameter.into_global_any(module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            let ty = self.symbol_type(symbol)?;
            parameters.push(self.lower_type_id(builder.tree_mut(), ty)?);
            symbols.push(symbol.local_id);
        }

        // resolve the checked return type from the function's own symbol
        let declared = self.symbol_type(symbol)?;
        let result = match self.signature_return(declared)? {
            Some(return_type) => self.lower_type_id(builder.tree_mut(), return_type)?,
            None => builder.tree_mut().insert(mir::Type::Void),
        };

        // declare the header under the function's name
        let Some(name) = self.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a name on one lowered function declaration"
                    .to_string(),
            });
        };
        let name = format!("{}.{}", self.local().path, self.strings.get(name));
        let mut header = builder.function_header(&name);
        for slot in 0..lifetimes.len() {
            header = header.lifetime(&format!("L{slot}"));
        }
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions.insert((symbol, Vec::new()), function);

        Ok(Body {
            function,
            has_this: false,
            parameters: symbols,
            lifetimes,
            substitution: FxIndexMap::default(),
            source: self.module,
            expression,
        })
    }

    /// Return whether one callable declares generic parameters beyond lifetimes.
    pub(in crate::lower) fn signature_is_generic(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // peel the callable down to its signature template
        let Ok((signature, owner)) = self.signature_of(ty) else {
            return Ok(false);
        };
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Ok(false);
        };

        // any non-lifetime parameter makes the callable instance-polymorphic
        let generics = &self.state(template.module_id)?.generics;
        let template = generics.get_template(template.local_id);
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the lifetime slot for each induced lifetime generic of one callable.
    pub(in crate::lower) fn signature_lifetimes(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<FxIndexMap<dir::LocalGenericParameterId, u16>> {
        // peel the callable down to its signature template
        let Ok((signature, owner)) = self.signature_of(ty) else {
            return Ok(FxIndexMap::default());
        };
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Ok(FxIndexMap::default());
        };

        // lifetime parameters take slots in declaration order
        let mut slots = FxIndexMap::default();
        let generics = &self.state(template.module_id)?.generics;
        let template = generics.get_template(template.local_id);
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime) {
                let slot = slots.len() as u16;
                slots.insert(*parameter, slot);
            }
        }

        Ok(slots)
    }

    /// Declare the MIR header for one class method with a body.
    pub(in crate::lower) fn declare_method(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        class: dir::LocalSymbolId,
        member: dir::LocalNodeId<dir::Member>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Body> {
        let module = self.module;

        // the sealed definition names the method symbol
        let node = member.into_global_any(module);
        let Some(symbol) = self.method_symbol(class.into_global(module), node)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one method declaration".to_string(),
            });
        };
        let lifetimes = self.signature_lifetimes(self.symbol_type(symbol)?)?;
        self.lifetime_slots = lifetimes.clone();

        // the receiver reference leads the parameter list
        let class = class.into_global(self.module);
        let Some(nominal) = self.nominals.get(&class) else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a lowered nominal for one method".to_string(),
            });
        };
        let (pointee, value) = (nominal.ty, nominal.value);

        // read the sealed receiver before borrowing the member signature
        let (signature, owner) = self.signature_of(self.symbol_type(symbol)?)?;
        let sealed_this = self.types(owner)?.signature(signature).this_parameter;

        // classify the member role in one narrow borrow
        let role = {
            let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a method body outside a method".to_string(),
                });
            };

            signature.role
        };

        // constructors initialize storage exclusively, whatever its final form
        let this = match role {
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
                let Some(sealed) = sealed_this else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR sealed an instance method without a receiver"
                            .to_string(),
                    });
                };

                self.lower_receiver(builder, sealed, pointee, value)?
            }
        };
        let parameter_nodes = {
            let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a method body outside a method".to_string(),
                });
            };

            signature.parameters.clone()
        };
        let mut parameters = vec![this];
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in &parameter_nodes {
            let node = parameter.into_global_any(module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            let ty = self.symbol_type(symbol)?;
            parameters.push(self.lower_type_id(builder.tree_mut(), ty)?);
            symbols.push(symbol.local_id);
        }

        // constructors initialize storage and return no value
        let result = match role {
            Some(dir::FunctionRole::Constructor) => builder.tree_mut().insert(mir::Type::Void),
            _ => {
                let declared = self.symbol_type(symbol)?;
                match self.signature_return(declared)? {
                    Some(return_type) => self.lower_type_id(builder.tree_mut(), return_type)?,
                    None => builder.tree_mut().insert(mir::Type::Void),
                }
            }
        };

        // declare the header under the class-qualified name
        let class_name = self
            .symbol_name(class)?
            .map(|name| self.strings.get(name).to_string())
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR declared a class without a name".to_string(),
            })?;
        let member_name = match self.local().bindings.get_symbol(symbol.local_id).name() {
            Some(name) => self.strings.get(name).to_string(),
            None if role == Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            None => {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a method without a name".to_string(),
                });
            }
        };
        let name = format!("{}.{class_name}.{member_name}", self.local().path);
        let mut header = builder.function_header(&name);
        for slot in 0..lifetimes.len() {
            header = header.lifetime(&format!("L{slot}"));
        }
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions.insert((symbol, Vec::new()), function);

        Ok(Body {
            function,
            has_this: true,
            parameters: symbols,
            lifetimes,
            substitution: FxIndexMap::default(),
            source: self.module,
            expression,
        })
    }

    /// Return the sealed method symbol declared at one member node.
    fn method_symbol(
        &self,
        owner: dir::GlobalSymbolId,
        member: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(definition) = self.definition(owner)? else {
            return Ok(None);
        };

        Ok(definition.method_declared_at(member))
    }

    /// Lower one sealed receiver type to the method's this parameter.
    ///
    /// The symbolic This base maps to the owner nominal: bare This receives
    /// the family's value position; borrowed forms wrap the declared storage.
    pub(in crate::lower) fn lower_receiver(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        sealed: dir::GlobalTypeId,
        pointee: mir::LocalNodeId<mir::Type>,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.ty(sealed)? {
            dir::Type::This => Ok(value),
            dir::Type::Form(form) => match form.form {
                // consuming receivers take the declared storage by value
                dir::Form::Owned => Ok(pointee),
                dir::Form::Borrowed(borrow) => {
                    let Some(borrow) = self.types(sealed.module_id)?.borrow_form_maybe(borrow)
                    else {
                        return Err(CompilerError::Internal {
                            message: "checked DIR is missing a receiver borrow form".to_string(),
                        });
                    };
                    let lifetime = self.borrow_lifetime(borrow.lifetime)?;
                    let access = self.borrow_access(borrow.access)?;

                    Ok(builder.tree_mut().insert(mir::Type::Reference {
                        kind: mir::ReferenceKind::Borrowed,
                        lifetime,
                        space: mir::Space::Local,
                        access,
                        pointee,
                        nullability: mir::Nullability::None,
                    }))
                }
                other => Err(crate::LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: format!("a '{other:?}' receiver form"),
                }
                .into()),
            },
            other => Err(crate::LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a '{}' receiver type", other.variant_name()),
            }
            .into()),
        }
    }
}
