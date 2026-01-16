use destack_dir::{Declaration, GlobalNodeId, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::{FunctionContext, LocalBinding, LowerError, LowerResult, Terminates};

use crate::lower::module::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a declaration into MIR.
    pub(crate) fn lower_declaration(
        &mut self,
        declaration_id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<()> {
        match declaration {
            Declaration::Function { .. } => {
                let _ = self.lower_function(declaration_id, declaration)?;
                Ok(())
            }

            // struct declarations: lower the type and its methods
            Declaration::Struct {
                descriptor,
                members,
                ..
            } => {
                let type_symbol = descriptor.symbol.into_global(self.module_id);

                // lower the struct type so it's cached
                let struct_mir_type =
                    if let Some(instance_type_id) = self.types.get_instance_type_id(type_symbol) {
                        Some(
                            self.type_lowerer.lower_type(
                                self.types,
                                instance_type_id,
                                self.module_id,
                                declaration_id
                                    .into_global_any(self.module_id)
                                    .into_anchored(Some(self.profile)),
                                &mut self.builder,
                            )?,
                        )
                    } else {
                        None
                    };

                // lower methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let Member::Method { .. } = member {
                        self.lower_method(*member_id, member, struct_mir_type, declaration_id)?;
                    }
                }

                Ok(())
            }

            // class declarations: lower the type and its methods
            Declaration::Class {
                descriptor,
                members,
                ..
            } => {
                let type_symbol = descriptor.symbol.into_global(self.module_id);

                // lower the nominal reference type for class methods
                let anchor = declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));
                let instance_type_id =
                    self.types
                        .get_instance_type_id(type_symbol)
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: anchor,
                            message: "class missing instance type".to_string(),
                        })?;
                if !self.type_lowerer.type_cache.contains_key(&instance_type_id) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "class instance layout not predeclared".to_string(),
                    });
                }
                let reference_type_id = self
                    .nominal_reference_type_id_for_symbol(type_symbol)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "class missing nominal reference type".to_string(),
                    })?;
                let class_mir_type = Some(self.type_lowerer.lower_type(
                    self.types,
                    reference_type_id,
                    self.module_id,
                    anchor,
                    &mut self.builder,
                )?);

                // lower methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let Member::Method { .. } = member {
                        self.lower_method(*member_id, member, class_mir_type, declaration_id)?;
                    }
                }

                Ok(())
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "unsupported declaration".to_string(),
            }),
        }
    }

    /// Lower a function declaration to a MIR function.
    pub(crate) fn lower_function(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        let Declaration::Function {
            descriptor,
            signature,
            body,
            ..
        } = declaration
        else {
            return Err(LowerError::UnsupportedConstruct {
                node: declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!(
                    "unsupported non-function declaration '{}'",
                    declaration.kind_name()
                ),
            })?;
        };

        let name_id =
            descriptor
                .name
                .map(|name| name.string())
                .ok_or(LowerError::UnsupportedConstruct {
                    node: declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing name".to_string(),
                })?;
        let name = self.compiler.program.strings.get(name_id).to_string();

        let symbol_id = descriptor.symbol.into_global(self.module_id);

        // return type
        let return_type = self.resolve_function_return_type(declaration_id)?;

        // parameter types
        let mut parameter_types = Vec::new();
        for parameter_id in &signature.dynamic_parameters {
            let parameter_node = GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty = self
                .types
                .get_declared_or_inferred_type_id(parameter_node)
                .ok_or(LowerError::MissingType {
                    node: parameter_node.into_anchored(Some(self.profile)),
                })?;
            let parameter_ty = self.type_lowerer.lower_type(
                self.types,
                parameter_ty,
                self.module_id,
                parameter_node.into_anchored(Some(self.profile)),
                &mut self.builder,
            )?;
            parameter_types.push(parameter_ty);
        }

        // extract return lifetime from @lifetime decorator before borrowing self.builder
        let return_lifetime = self.extract_lifetime_annotation(descriptor.symbol, signature);

        // build the function
        let mut builder = self.builder.function(&name, &parameter_types, return_type);
        let function_id = builder.function_id();
        self.functions_by_symbol.insert(symbol_id, function_id);

        // set return lifetime
        builder.set_return_lifetime(return_lifetime);

        let mut function_ctx = FunctionContext::new(
            self.module_id,
            self.profile,
            &self.compiler.program,
            self.dir_tree,
            self.symbols,
            self.types,
            &self.compiler.program.strings,
            &self.functions_by_symbol,
            &self.globals_by_symbol,
            &self.type_lowerer,
            builder,
        );

        // create entry block
        let entry_block = function_ctx.builder.create_block();
        function_ctx.builder.switch_to_block(entry_block);

        // add parameter locals
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol_id = parameter.symbol().into_global(self.module_id);
            let ty = parameter_types[index];
            let variable = function_ctx.builder.create_variable(ty);
            let value = function_ctx.builder.function_parameter(index);
            function_ctx.builder.define_variable(variable, value);
            function_ctx
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_ctx.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_ctx.builder.return_(None);
                } else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: declaration_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "missing terminator".to_string(),
                    })?;
                }
            }
        } else {
            function_ctx.builder.return_(None);
        }

        function_ctx.builder.finish();
        Ok(function_id)
    }

    /// Resolve a function return type for lowering.
    fn resolve_function_return_type(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let node_id = declaration_id.into_global_any(self.module_id);

        let signature_type_id =
            self.types
                .get_inferred_type_id(node_id)
                .ok_or(LowerError::MissingType {
                    node: node_id.into_anchored(Some(self.profile)),
                })?;
        let return_type_id = match self.types.get_type(signature_type_id) {
            dir::Type::Function { return_type, .. } => {
                return_type.ok_or(LowerError::MissingType {
                    node: node_id.into_anchored(Some(self.profile)),
                })?
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: node_id.into_anchored(Some(self.profile)),
                    message: "missing function signature type".to_string(),
                })?;
            }
        };

        self.type_lowerer.lower_type(
            self.types,
            return_type_id,
            self.module_id,
            node_id.into_anchored(Some(self.profile)),
            &mut self.builder,
        )
    }

    /// Lower a method member to a MIR function.
    fn lower_method(
        &mut self,
        member_id: LocalNodeId<Member>,
        member: &Member,
        this_type: Option<mir::LocalNodeId<mir::Type>>,
        parent_declaration_id: LocalNodeId<Declaration>,
    ) -> LowerResult<()> {
        // require a method member
        let Member::Method {
            key,
            signature,
            body,
            symbol,
            ..
        } = member
        else {
            return Ok(());
        };

        // decide whether this method is a constructor
        let is_constructor = matches!(
            signature.mode,
            Some(dir::FunctionMode::Constructor) | Some(dir::FunctionMode::New)
        );

        // resolve the method name
        let name_str = if is_constructor {
            // reject constructor keys
            if key.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "constructor cannot have a name".to_string(),
                });
            }

            // resolve the nominal declaration descriptor
            let declaration = self.dir_tree.get(parent_declaration_id);
            let descriptor =
                self.descriptor_for_declaration_or_error(parent_declaration_id, declaration)?;

            // require a declaration name for constructor
            let name = descriptor
                .name
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: parent_declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "constructor must have a declaration name".to_string(),
                })?;

            // format the constructor name
            let type_name = self.compiler.program.strings.get(name.string()).to_string();
            format!("{type_name}.constructor")
        } else {
            // resolve the static method key
            let method_name = self.member_name_or_error(key.as_ref(), member_id)?;
            self.compiler.program.strings.get(method_name).to_string()
        };

        // resolve the method symbol
        let method_symbol = symbol.into_global(self.module_id);

        // resolve return type from the method's inferred signature
        let member_node = member_id.into_global_any(self.module_id);
        let return_type = self.resolve_method_return_type(member_id, member_node)?;

        // initialize parameter types
        let mut parameter_types = Vec::new();

        // capture this type for constructor initialization
        let constructor_this_type = if is_constructor {
            // require an instance type for constructors
            Some(this_type.ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "constructor missing instance type".to_string(),
                }
            })?)
        }
        // skip constructor state for non constructors
        else {
            None
        };

        // add this parameter when lowering an instance method
        if !is_constructor && let Some(this_ty) = this_type {
            parameter_types.push(this_ty);
        }

        // lower declared parameter types
        for parameter_id in &signature.dynamic_parameters {
            // resolve the parameter type id
            let parameter_node = GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty_id = self
                .types
                .get_declared_or_inferred_type_id(parameter_node)
                .ok_or(LowerError::MissingType {
                    node: parameter_node.into_anchored(Some(self.profile)),
                })?;

            // lower the parameter type
            let parameter_ty = self.type_lowerer.lower_type(
                self.types,
                parameter_ty_id,
                self.module_id,
                parameter_node.into_anchored(Some(self.profile)),
                &mut self.builder,
            )?;
            parameter_types.push(parameter_ty);
        }

        // build the function
        let builder = self
            .builder
            .function(&name_str, &parameter_types, return_type);
        let function_id = builder.function_id();

        // register by symbol for direct calls via resolution
        self.functions_by_symbol.insert(method_symbol, function_id);

        // collect function context dependencies
        let program = &self.compiler.program;
        let strings = &program.strings;

        // create function context
        let mut function_ctx = FunctionContext::new(
            self.module_id,
            self.profile,
            program,
            self.dir_tree,
            self.symbols,
            self.types,
            strings,
            &self.functions_by_symbol,
            &self.globals_by_symbol,
            &self.type_lowerer,
            builder,
        );

        // create entry block
        let entry_block = function_ctx.builder.create_block();
        function_ctx.builder.switch_to_block(entry_block);

        // initialize constructor state before parameter locals
        if let Some(this_ty) = constructor_this_type {
            // resolve the constructor anchor
            let node = member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));

            // select the layout type
            let layout_type = match function_ctx.builder.tree().get(this_ty) {
                mir::Type::Reference { pointee, .. } => *pointee,
                _ => this_ty,
            };

            // initialize constructor state
            let layout = self
                .type_lowerer
                .layout_for_type_or_error(layout_type, node)?;
            function_ctx.start_constructor(this_ty, layout.clone(), node)?;
        }

        // track parameter index for locals
        let mut param_index = 0;

        // add this parameter as first local for instance methods
        if let Some(this_ty) = this_type
            && !is_constructor
        {
            let this_variable = function_ctx.builder.create_variable(this_ty);
            let this_value = function_ctx.builder.function_parameter(param_index);
            function_ctx
                .builder
                .define_variable(this_variable, this_value);

            // set this binding for Expression::This lookup
            function_ctx.this_binding = Some(LocalBinding {
                variable: this_variable,
                ty: this_ty,
            });

            // advance the parameter index
            param_index += 1;
        }

        // add declared parameter locals
        for parameter_id in &signature.dynamic_parameters {
            // resolve the parameter symbol
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol_id = parameter.symbol().into_global(self.module_id);

            // bind the parameter local
            let ty = parameter_types[param_index];
            let variable = function_ctx.builder.create_variable(ty);
            let value = function_ctx.builder.function_parameter(param_index);
            function_ctx.builder.define_variable(variable, value);
            function_ctx
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });

            // advance the parameter index
            param_index += 1;
        }

        // lower the body when present
        if let Some(body_id) = body {
            // lower the body and handle fallthrough
            let terminated = function_ctx.lower_body(*body_id)?;
            if terminated == Terminates::No {
                // return constructed value when constructor falls through
                if is_constructor {
                    let node = member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile));
                    function_ctx.return_constructor_value(node)?;
                }
                // return void when allowed
                else if return_type == self.type_lowerer.ty_void {
                    function_ctx.builder.return_(None);
                }
                // error on missing terminator
                else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: parent_declaration_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "method body missing terminator".to_string(),
                    });
                }
            }
        }
        // synthesize constructor return when body is missing
        else if is_constructor {
            let node = member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            function_ctx.return_constructor_value(node)?;
        }
        // synthesize void return when body is missing
        else {
            function_ctx.builder.return_(None);
        }

        // finish the function builder
        function_ctx.builder.finish();

        Ok(())
    }

    /// Resolve a method return type for lowering.
    fn resolve_method_return_type(
        &mut self,
        member_id: LocalNodeId<Member>,
        member_node: dir::GlobalNodeIdAny,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // get signature type from inferred types
        let signature_type_id = self
            .types
            .get_inferred_type_id(member_node)
            .ok_or_else(|| LowerError::MissingType {
                node: member_node.into_anchored(Some(self.profile)),
            })?;

        // extract return type from function signature
        let dir::Type::Function { return_type, .. } = self.types.get_type(signature_type_id) else {
            return Err(LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "method signature is not a function type".to_string(),
            });
        };

        // lower return type (or void if none)
        let Some(return_type_id) = return_type else {
            return Ok(self.type_lowerer.ty_void);
        };

        self.type_lowerer.lower_type(
            self.types,
            *return_type_id,
            self.module_id,
            member_node.into_anchored(Some(self.profile)),
            &mut self.builder,
        )
    }
}
