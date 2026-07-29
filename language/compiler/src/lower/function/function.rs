use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    FunctionDefinition, GenericInstanceKey, LifetimeParameters, ModuleLowerer, ReceiverBinding,
    TypeSubstitution,
};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the MIR header for one function declaration with a body.
    pub(in crate::lower) fn declare_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<FunctionDefinition> {
        let module = self.module;

        // declare the signature's lifetime generics before its parameter types
        let node = declaration.into_global_any(module);
        let Some(symbol) = self.symbol_declared_at(node)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one function declaration".to_string(),
            });
        };
        let declared = self.symbol_type(symbol)?;
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let type_substitution = TypeSubstitution::default();

        // resolve the checked parameter types alongside their symbols
        let dir::Declaration::Function(function) = self.local().tree().get(declaration) else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a function body outside a function".to_string(),
            });
        };
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(symbol.local_id);
        }

        // lower the sealed signature through the shared callable path
        let signature =
            self.lower_signature(builder, declared, &type_substitution, &lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "checked DIR function parameters disagree with its sealed signature"
                    .to_string(),
            });
        }

        // declare the header under the function's name
        let Some(name) = self.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a name on one lowered function declaration"
                    .to_string(),
            });
        };
        let name = format!("{}.{}", self.local().path, self.strings.get(name));
        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.declare_function(header);
        self.functions
            .insert(GenericInstanceKey::non_generic(symbol), function);

        Ok(FunctionDefinition {
            function,
            has_this: false,
            parameters: symbols,
            type_substitution,
            lifetime_parameters,
            source: self.module,
            expression,
        })
    }

    /// Return whether one callable declares type parameters.
    pub(in crate::lower) fn signature_has_instance_parameters(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // peel the callable down to its signature template
        let (signature, owner) = self.signature(ty)?;
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Ok(false);
        };

        // any non-lifetime parameter makes the callable instance-polymorphic
        let template_module = template.module_id;
        let generics = &self.state(template_module)?.generics;
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
    pub(in crate::lower) fn lifetime_parameters(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<LifetimeParameters> {
        // peel the callable down to its signature template
        let (signature, owner) = self.signature(ty)?;
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Ok(LifetimeParameters::default());
        };

        LifetimeParameters::from_template(self, template)
    }

    /// Declare the MIR header for one member callable with a body.
    pub(in crate::lower) fn declare_method(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        member: dir::LocalNodeId<dir::Member>,
        expression: dir::LocalNodeId<dir::Expression>,
        is_static: bool,
    ) -> CompilerResult<FunctionDefinition> {
        let pointer_bytes = builder.pointer_bytes();
        let declared = self.symbol_type(symbol)?;
        let lifetime_parameters = self.lifetime_parameters(declared)?;

        // read the extension target when the owner is an extension
        let extension_target = match self.definition(owner)? {
            Some(dir::Definition::Extension(extension)) => Some(extension.target),
            _ => None,
        };

        // receive at the extension target, or at the nominal itself
        let receiver = match extension_target {
            Some(target) => ReceiverBinding::Type(target.r#type()),
            None => ReceiverBinding::Application(dir::GenericApplication {
                symbol: owner,
                arguments: dir::TypeListId::EMPTY,
            }),
        };
        let type_substitution = TypeSubstitution::default().with_receiver(receiver);

        // read the sealed receiver before borrowing the member signature
        let (signature, signature_module) = self.signature(declared)?;
        let sealed_this = self
            .types(signature_module)?
            .signature(signature)
            .this_parameter;

        // classify the member role in one narrow borrow
        let role = {
            let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a method body outside a method".to_string(),
                });
            };

            signature.role
        };

        let this = match role {
            // static members take no receiver
            _ if is_static => None,
            // constructors initialize storage exclusively, whatever its final form
            Some(dir::FunctionRole::Constructor) => {
                let nominal = self
                    .type_lowerer(
                        builder.tree_mut(),
                        pointer_bytes,
                        &type_substitution,
                        &lifetime_parameters,
                    )
                    .lower_nominal(owner, &[])?;

                Some(builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime: mir::Lifetime::empty(),
                    space: mir::Space::Local,
                    access: mir::Access::Exclusive,
                    pointee: nominal.storage,
                    nullability: mir::Nullability::None,
                }))
            }
            // methods receive this at their sealed receiver type
            _ => {
                let Some(this_type) = sealed_this else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR sealed an instance method without a receiver"
                            .to_string(),
                    });
                };

                Some(
                    self.type_lowerer(
                        builder.tree_mut(),
                        pointer_bytes,
                        &type_substitution,
                        &lifetime_parameters,
                    )
                    .lower(this_type)?,
                )
            }
        };
        let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a method body outside a method".to_string(),
            });
        };
        let parameter_nodes = signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(self.module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(symbol.local_id);
        }

        // lower the sealed signature and prepend its receiver
        let signature =
            self.lower_signature(builder, declared, &type_substitution, &lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "checked DIR method parameters disagree with its sealed signature"
                    .to_string(),
            });
        }
        let mut parameters = signature.parameters;
        if let Some(this) = this {
            parameters.insert(0, this);
        }

        // constructors initialize storage and return no value
        let result = match role {
            Some(dir::FunctionRole::Constructor) => builder.tree_mut().intern_type(mir::Type::Void),
            _ => signature.result,
        };

        // qualify anonymous extensions by their target root
        let mut owner_name = self.symbol_name(owner)?;
        if owner_name.is_none()
            && let Some(root) = extension_target.and_then(|target| target.root())
        {
            owner_name = self.symbol_name(root)?;
        }
        let Some(owner_name) = owner_name else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a member owner without a name".to_string(),
            });
        };
        let owner_name = self.strings.get(owner_name).to_string();
        let member_name = match self.local().bindings.get_symbol(symbol.local_id).name() {
            Some(name) => self.strings.get(name).to_string(),
            None if role == Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            None => {
                return Err(CompilerError::Internal {
                    message: "checked DIR declared a method without a name".to_string(),
                });
            }
        };

        // split the accessor pair that shares one member name by role
        let member_name = match role {
            Some(dir::FunctionRole::Getter) => format!("{member_name}.get"),
            Some(dir::FunctionRole::Setter) => format!("{member_name}.set"),
            _ => member_name,
        };
        let name = format!("{}.{owner_name}.{member_name}", self.local().path);
        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions
            .insert(GenericInstanceKey::non_generic(symbol), function);

        Ok(FunctionDefinition {
            function,
            has_this: !is_static,
            parameters: symbols,
            type_substitution,
            lifetime_parameters,
            source: self.module,
            expression,
        })
    }

    /// Return the sealed method symbol declared at one member node.
    pub(in crate::lower) fn method_symbol(
        &self,
        owner: dir::GlobalSymbolId,
        member: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(definition) = self.definition(owner)? else {
            return Ok(None);
        };

        Ok(definition.method_declared_at(member))
    }
}
