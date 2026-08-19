use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    FunctionDeclaration, FunctionDefinition, GenericInstanceKey, LifetimeParameters, ModuleLowerer,
    constructor_receiver_type,
};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the header for one function declaration with a body.
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
                message: "missing a symbol for one function declaration".to_string(),
            });
        };
        let declared = self.symbol_type(symbol)?;
        let lifetime_parameters = self.lifetime_parameters(declared)?;

        // resolve the parameter types alongside their symbols
        let dir::Declaration::Function(function) = self.local().tree().get(declaration) else {
            return Err(CompilerError::Internal {
                message: "a function body declared outside a function".to_string(),
            });
        };
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(self.local().tree().get(parameter).default_value());
            let node = parameter.into_global_any(module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(symbol.local_id);
        }

        // lower the signature through the shared callable path
        let (parameters, result) =
            self.lower_signature(builder, declared, None, &lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "function parameters disagree with the declared signature".to_string(),
            });
        }

        // declare the header under the function's lexical path
        let name = self.symbol_path(symbol)?;
        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions.insert(
            GenericInstanceKey::non_generic(symbol),
            FunctionDeclaration::Declared(function),
        );

        Ok(FunctionDefinition {
            function,
            symbol,
            has_this: false,
            parameters: symbols,
            instance: None,
            lifetime_parameters,
            source: self.module,
            expression,
            constructs: None,
            defaults,
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

        self.template_has_instance_parameters(template)
    }

    /// Return whether one owner declaration binds type parameters of its own.
    pub(in crate::lower) fn owner_has_instance_parameters(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        // read the owner template, treating non-nominal owners as concrete
        let Some(definition) = self.definition(owner)? else {
            return Ok(false);
        };
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        self.template_has_instance_parameters(template.into_global(owner.module_id))
    }

    /// Return whether one generic template declares any non-lifetime parameter.
    fn template_has_instance_parameters(
        &self,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<bool> {
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

    /// Declare the header for one member callable with a body.
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

        // read the declared receiver before borrowing the member signature
        let (signature, signature_module) = self.signature(declared)?;
        let declared_this = self
            .types(signature_module)?
            .signature(signature)
            .this_parameter;

        // classify the member role in one narrow borrow
        let role = {
            let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
                return Err(CompilerError::Internal {
                    message: "a method body declared outside a method".to_string(),
                });
            };

            signature.role
        };

        // classify the receiver the member declares
        let this = match role {
            // take no receiver for static members
            _ if is_static => None,
            // receive an exclusive borrow of uninitialized storage
            Some(dir::FunctionRole::Constructor) => {
                let nominal = self
                    .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters)
                    .lower_nominal(owner, &[])?;

                Some(constructor_receiver_type(
                    builder.tree_mut(),
                    nominal.storage,
                ))
            }
            // pass this at the declared receiver type
            _ => {
                let Some(this_type) = declared_this else {
                    return Err(CompilerError::Internal {
                        message: "an instance method without a receiver".to_string(),
                    });
                };

                Some(
                    self.type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters)
                        .lower(this_type)?,
                )
            }
        };
        let dir::Member::Method { signature, .. } = self.local().tree().get(member) else {
            return Err(CompilerError::Internal {
                message: "a method body declared outside a method".to_string(),
            });
        };
        let parameter_nodes = signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(self.local().tree().get(parameter).default_value());
            let node = parameter.into_global_any(self.module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(symbol.local_id);
        }

        // lower the signature and prepend its receiver
        let (mut parameters, result) =
            self.lower_signature(builder, declared, None, &lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "method parameters disagree with the declared signature".to_string(),
            });
        }
        if let Some(this) = this {
            parameters.insert(0, this);
        }

        // give constructors a void result
        let result = match role {
            Some(dir::FunctionRole::Constructor) => builder.tree_mut().intern_type(mir::Type::Void),
            _ => result,
        };

        let member_name = self.member_extern_name(symbol, owner, role)?;
        let name = format!("{}.{member_name}", self.local().path);
        let header = lifetime_parameters.declare(builder.function_header(&name));
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions.insert(
            GenericInstanceKey::non_generic(symbol),
            FunctionDeclaration::Declared(function),
        );

        Ok(FunctionDefinition {
            function,
            symbol,
            has_this: !is_static,
            parameters: symbols,
            instance: None,
            lifetime_parameters,
            source: self.module,
            expression,
            constructs: (role == Some(dir::FunctionRole::Constructor)).then_some(owner),
            defaults,
        })
    }

    /// Return the method symbol declared at one member node.
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

impl ModuleLowerer<'_> {
    /// Return one member's owner-qualified extern name.
    pub(in crate::lower) fn member_extern_name(
        &self,
        symbol: dir::GlobalSymbolId,
        owner: dir::GlobalSymbolId,
        role: Option<dir::FunctionRole>,
    ) -> CompilerResult<String> {
        // qualify anonymous extensions by their target root
        let extension_root = match self.definition(owner)? {
            Some(dir::Definition::Extension(extension)) => extension.target.root(),
            _ => None,
        };
        let mut owner_name = self.symbol_name(owner)?;
        if owner_name.is_none()
            && let Some(root) = extension_root
        {
            owner_name = self.symbol_name(root)?;
        }
        let Some(owner_name) = owner_name else {
            return Err(CompilerError::Internal {
                message: "a member owner without a name".to_string(),
            });
        };
        let owner_name = self.strings.get(owner_name).to_string();
        let member_name = match self.symbol_name(symbol)? {
            Some(name) => self.strings.get(name).to_string(),
            None if role == Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            None => {
                return Err(CompilerError::Internal {
                    message: "a method without a name".to_string(),
                });
            }
        };

        // split the accessor pair that shares one member name by role
        let member_name = match role {
            Some(dir::FunctionRole::Getter) => format!("{member_name}.get"),
            Some(dir::FunctionRole::Setter) => format!("{member_name}.set"),
            _ => member_name,
        };

        Ok(format!("{owner_name}.{member_name}"))
    }
}
