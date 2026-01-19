use destack_dir::{Declaration, GlobalNodeId, GlobalNodeIdAny, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::{
    FunctionContext, FunctionEnv, FunctionState, LocalBinding, LowerError, LowerResult, Terminates,
};

use crate::lower::module::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve the signature type id for a declaration or member node.
    pub(crate) fn signature_type_id_for_node(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve the signature type id
        self.types
            .get_signature_type_for_node(node_id)
            .ok_or(LowerError::MissingType {
                node: node_id.into_anchored(Some(self.profile)),
            })
    }

    /// Resolve the MIR signature type for a declaration or member node.
    pub(crate) fn signature_mir_type_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the signature type id
        let signature_type_id = self.signature_type_id_for_node(node_id)?;

        // lower the signature type
        self.lower_type(signature_type_id, node_id.into_anchored(Some(self.profile)))
    }

    /// Lower a function declaration to a MIR function.
    pub(crate) fn lower_function(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // require a function declaration
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

        // resolve function name and symbol
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

        // resolve return type
        let return_type = self.resolve_function_return_type(declaration_id)?;

        // resolve parameter types
        let mut parameter_types = Vec::new();
        for parameter_id in &signature.dynamic_parameters {
            let parameter_node = GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty = self
                .types
                .get_declared_or_inferred_type_id(parameter_node)
                .ok_or(LowerError::MissingType {
                    node: parameter_node.into_anchored(Some(self.profile)),
                })?;
            let parameter_ty = self.lower_type(
                parameter_ty,
                parameter_node.into_anchored(Some(self.profile)),
            )?;
            parameter_types.push(parameter_ty);
        }

        // extract return lifetime from @lifetime decorator
        let return_lifetime = self.extract_lifetime_annotation(descriptor.symbol, signature);

        // resolve the signature type for direct callsites
        let signature_type =
            self.signature_mir_type_for_node(declaration_id.into_global_any(self.module_id))?;

        // build the function
        let mut builder = self.builder.function(&name, &parameter_types, return_type);
        let function_id = builder.function_id();
        self.functions_by_symbol.insert(symbol_id, function_id);
        self.function_signature_types
            .insert(function_id, signature_type);

        // set return lifetime
        builder.set_return_lifetime(return_lifetime);

        // build function env
        let env = FunctionEnv {
            module_id: self.module_id,
            profile: self.profile,
            program: &self.compiler.program,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            strings: &self.compiler.program.strings,
            functions_by_symbol: &self.functions_by_symbol,
            function_signature_types: &self.function_signature_types,
            globals_by_symbol: &self.globals_by_symbol,
            interface_slots_by_symbol: &self.interface_slots_by_symbol,
            interface_itab_ids: &self.interface_itab_ids,
            virtual_method_slots_by_symbol: &self.virtual_method_slots_by_symbol,
            vtable_globals_by_symbol: &self.vtable_globals_by_symbol,
            dispatch_call_name: self.dispatch_call_name,
            dispatch_construct_name: self.dispatch_construct_name,
            type_lowerer: &self.type_lowerer,
        };
        let state = FunctionState::new(builder);
        let mut function_ctx = FunctionContext::new(env, state);

        // create entry block
        let entry_block = function_ctx.state.builder.create_block();
        function_ctx.state.builder.switch_to_block(entry_block);

        // add parameter locals
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol_id = parameter.symbol().into_global(self.module_id);
            let ty = parameter_types[index];
            let variable = function_ctx.state.builder.create_variable(ty);
            let value = function_ctx.state.builder.function_parameter(index);
            function_ctx.state.builder.define_variable(variable, value);
            function_ctx
                .state
                .bindings
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_ctx.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_ctx.state.builder.return_(None);
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
            function_ctx.state.builder.return_(None);
        }

        // finish the function builder
        function_ctx.state.builder.finish();
        Ok(function_id)
    }

    /// Resolve a function return type for lowering.
    fn resolve_function_return_type(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the declaration node id
        let node_id = declaration_id.into_global_any(self.module_id);

        // resolve the function signature type
        let signature_type_id = self.signature_type_id_for_node(node_id)?;

        // extract the return type id from the signature
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

        // lower the return type
        self.lower_type(return_type_id, node_id.into_anchored(Some(self.profile)))
    }

    /// Lower a method member to a MIR function.
    pub(crate) fn lower_method(
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

        // track constructor declaration symbol when needed
        let mut constructor_symbol = None;

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
            constructor_symbol = Some(descriptor.symbol.into_global(self.module_id));

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
            // resolve the static method key or dispatch name
            let method_name =
                self.member_dispatch_name_or_error(key.as_ref(), signature.mode, member_id)?;
            self.compiler.program.strings.get(method_name).to_string()
        };

        // resolve the method symbol
        let method_symbol = symbol.into_global(self.module_id);

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

        // resolve parameter types
        let parameter_types = self.method_parameter_types(signature, this_type)?;

        // resolve return type from the method's inferred signature
        let member_node = member_id.into_global_any(self.module_id);
        let return_type = if let Some(constructor_type) = constructor_this_type {
            constructor_type
        } else {
            self.resolve_method_return_type(member_id, member_node)?
        };

        // resolve the signature type for direct callsites
        let signature_type =
            self.signature_mir_type_for_node(member_id.into_global_any(self.module_id))?;

        // build the function
        let builder = self
            .builder
            .function(&name_str, &parameter_types, return_type);
        let function_id = builder.function_id();

        // register by symbol for direct calls via resolution
        self.functions_by_symbol.insert(method_symbol, function_id);
        self.function_signature_types
            .insert(function_id, signature_type);

        // create function context
        let env = FunctionEnv {
            module_id: self.module_id,
            profile: self.profile,
            program: &self.compiler.program,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            strings: &self.compiler.program.strings,
            functions_by_symbol: &self.functions_by_symbol,
            function_signature_types: &self.function_signature_types,
            globals_by_symbol: &self.globals_by_symbol,
            interface_slots_by_symbol: &self.interface_slots_by_symbol,
            interface_itab_ids: &self.interface_itab_ids,
            virtual_method_slots_by_symbol: &self.virtual_method_slots_by_symbol,
            vtable_globals_by_symbol: &self.vtable_globals_by_symbol,
            dispatch_call_name: self.dispatch_call_name,
            dispatch_construct_name: self.dispatch_construct_name,
            type_lowerer: &self.type_lowerer,
        };
        let state = FunctionState::new(builder);
        let mut function_ctx = FunctionContext::new(env, state);

        // create entry block
        let entry_block = function_ctx.state.builder.create_block();
        function_ctx.state.builder.switch_to_block(entry_block);

        // initialize constructor state before parameter locals
        if let Some(this_ty) = constructor_this_type {
            // resolve the constructor anchor
            let node = member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));

            // select the layout type
            let layout_type = match function_ctx.state.builder.tree().get(this_ty) {
                mir::Type::Reference { pointee, .. } => *pointee,
                _ => this_ty,
            };

            // initialize constructor state
            let layout = self
                .type_lowerer
                .layout_for_type_or_error(layout_type, node)?;
            let class_symbol =
                constructor_symbol.filter(|symbol| symbol.ty() == dir::SymbolType::Class);
            function_ctx.initialize_constructor(this_ty, layout.clone(), node, class_symbol)?;
        }

        // track parameter index for locals
        let mut param_index = 0;

        // add 'this' parameter as first local for instance methods
        if let Some(this_ty) = this_type
            && !is_constructor
        {
            let this_variable = function_ctx.state.builder.create_variable(this_ty);
            let this_value = function_ctx.state.builder.function_parameter(param_index);
            function_ctx
                .state
                .builder
                .define_variable(this_variable, this_value);

            // set 'this' binding for Expression::This lookup
            function_ctx.state.bindings.this_binding = Some(LocalBinding {
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
            let variable = function_ctx.state.builder.create_variable(ty);
            let value = function_ctx.state.builder.function_parameter(param_index);
            function_ctx.state.builder.define_variable(variable, value);
            function_ctx
                .state
                .bindings
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
                    function_ctx.state.builder.return_(None);
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
            function_ctx.state.builder.return_(None);
        }

        // finish the function builder
        function_ctx.state.builder.finish();

        Ok(())
    }

    /// Resolve a method return type for lowering.
    pub(crate) fn resolve_method_return_type(
        &mut self,
        member_id: LocalNodeId<Member>,
        member_node: dir::GlobalNodeIdAny,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // get signature type from analyzed metadata
        let signature_type_id = self.signature_type_id_for_node(member_node)?;

        // extract return type from function signature
        let dir::Type::Function { return_type, .. } = self.types.get_type(signature_type_id) else {
            return Err(LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "method signature is not a function type".to_string(),
            });
        };

        // lower return type or default to void
        let Some(return_type_id) = return_type else {
            return Ok(self.type_lowerer.ty_void);
        };

        // lower the return type
        self.lower_type(
            *return_type_id,
            member_node.into_anchored(Some(self.profile)),
        )
    }

    /// Resolve parameter types for a method signature.
    pub(crate) fn method_parameter_types(
        &mut self,
        signature: &dir::FunctionSignature,
        this_type: Option<mir::LocalNodeId<mir::Type>>,
    ) -> LowerResult<Vec<mir::LocalNodeId<mir::Type>>> {
        // decide whether this method is a constructor
        let is_constructor = matches!(
            signature.mode,
            Some(dir::FunctionMode::Constructor) | Some(dir::FunctionMode::New)
        );

        // initialize parameter types
        let mut parameter_types = Vec::new();

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
            let parameter_ty = self.lower_type(
                parameter_ty_id,
                parameter_node.into_anchored(Some(self.profile)),
            )?;
            parameter_types.push(parameter_ty);
        }

        Ok(parameter_types)
    }
}
