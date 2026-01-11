use std::collections::HashMap;

use destack_dir::{Declaration, DynamicKey, GlobalNodeId, LocalNodeId, Member};
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

                // lower the class type so it's cached
                let class_mir_type =
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
        let return_lifetime = self.extract_lifetime_annotation(declaration_id, signature);

        // build the function
        let mut builder = self.builder.function(&name, &parameter_types, return_type);
        let function_id = builder.function_id();
        self.functions_by_symbol.insert(symbol_id, function_id);

        // set return lifetime
        builder.set_return_lifetime(return_lifetime);

        let mut function_ctx = FunctionContext::new(
            self.module_id,
            self.profile,
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

        // get method name from key
        let method_name = match key {
            Some(DynamicKey::Name(name)) => *name,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "method must have a static name".to_string(),
                });
            }
        };

        let method_symbol = symbol.into_global(self.module_id);
        let name_str = self.compiler.program.strings.get(method_name).to_string();

        // resolve return type from the method's inferred signature
        let member_node = member_id.into_global_any(self.module_id);
        let return_type = self.resolve_method_return_type(member_id, member_node)?;

        // build parameter types: this + declared parameters
        let mut parameter_types = Vec::new();

        // add this parameter if we have a type for it
        if let Some(this_ty) = this_type {
            parameter_types.push(this_ty);
        }

        // add declared parameters
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

        // build the function
        let builder = self
            .builder
            .function(&name_str, &parameter_types, return_type);
        let function_id = builder.function_id();

        // register by symbol for direct calls via Resolution
        self.functions_by_symbol.insert(method_symbol, function_id);

        // create function lowerer
        let mut function_ctx = FunctionContext::new(
            self.module_id,
            self.profile,
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

        // add this parameter as first local (if present)
        let mut param_index = 0;
        if let Some(this_ty) = this_type {
            let this_variable = function_ctx.builder.create_variable(this_ty);
            let this_value = function_ctx.builder.function_parameter(param_index);
            function_ctx
                .builder
                .define_variable(this_variable, this_value);

            // set this_binding for Expression::This lookup
            function_ctx.this_binding = Some(LocalBinding {
                variable: this_variable,
                ty: this_ty,
            });
            param_index += 1;
        }

        // add declared parameter locals
        for parameter_id in &signature.dynamic_parameters {
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol_id = parameter.symbol().into_global(self.module_id);
            let ty = parameter_types[param_index];
            let variable = function_ctx.builder.create_variable(ty);
            let value = function_ctx.builder.function_parameter(param_index);
            function_ctx.builder.define_variable(variable, value);
            function_ctx
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });
            param_index += 1;
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_ctx.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_ctx.builder.return_(None);
                } else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: parent_declaration_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "method body missing terminator".to_string(),
                    });
                }
            }
        } else {
            function_ctx.builder.return_(None);
        }

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

    /// Extract return lifetime from @lifetime decorator annotations on a function.
    ///
    /// Supports:
    /// - `@lifetime("static")` - static lifetime
    /// - `@lifetime(param1, param2, ...)` - borrows from named parameters
    fn extract_lifetime_annotation(
        &self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        signature: &dir::FunctionSignature,
    ) -> mir::Lifetime {
        let annotations = self.dir_tree.get_annotations(declaration_id.id);

        for annotation_id in annotations {
            let annotation = self.dir_tree.get(annotation_id);

            // look for decorator annotations
            let dir::Annotation::Decorator {
                left, arguments, ..
            } = annotation
            else {
                continue;
            };

            // check if the decorator is named "lifetime"
            let left_expr = self.dir_tree.get(*left);
            let is_lifetime = match left_expr {
                dir::Expression::UnresolvedPath { path, .. }
                | dir::Expression::LocalReference { path, .. }
                | dir::Expression::ModuleReference { path, .. }
                | dir::Expression::GlobalReference { path, .. } => {
                    if let Some(first) = path.first_segment() {
                        self.compiler.program.strings.get(first) == "lifetime"
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !is_lifetime {
                continue;
            }

            // parse the arguments
            let Some(args) = arguments else {
                // @lifetime with no args: defaults to inferred
                continue;
            };
            if args.is_empty() {
                continue;
            }

            // check for @lifetime("static")
            if args.len() == 1 {
                let arg = self.dir_tree.get(args[0]);
                if let dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } = arg
                {
                    let expr = self.dir_tree.get(*value);
                    if let dir::Expression::ScalarLiteral {
                        value: dir::ScalarLiteral::String(string_id),
                    } = expr
                        && self.compiler.program.strings.get(*string_id) == "static"
                    {
                        return mir::Lifetime::Static;
                    }
                }
            }

            // build mapping from parameter names to indices
            let mut param_name_to_index: HashMap<String, u32> = HashMap::new();
            for (index, param_id) in signature.dynamic_parameters.iter().enumerate() {
                let param: &dir::Parameter = self.dir_tree.get(*param_id);
                let param_name = match param {
                    dir::Parameter::Named { name, .. } => Some(*name),
                    dir::Parameter::Variadic { name, .. } => Some(*name),
                    dir::Parameter::Pattern { .. } => None,
                };
                if let Some(name_id) = param_name {
                    let name_str = self.compiler.program.strings.get(name_id).to_string();
                    param_name_to_index.insert(name_str, index as u32);
                }
            }

            // parse parameter references from arguments
            let mut param_indices = Vec::new();
            for arg_id in args {
                let arg = self.dir_tree.get(*arg_id);
                let value_id = match arg {
                    dir::Argument::Positional { value } => value,
                    dir::Argument::Named { value, .. } => value,
                    dir::Argument::Labeled { value, .. } => value,
                    dir::Argument::Spread { value, .. } => value,
                };
                let expr = self.dir_tree.get(*value_id);

                // look for identifier references that match parameter names
                let param_name: Option<String> = match expr {
                    // single-segment path is a parameter reference
                    dir::Expression::UnresolvedPath { path, .. }
                    | dir::Expression::LocalReference { path, .. } => {
                        if path.segments.len() == 1 {
                            Some(
                                self.compiler
                                    .program
                                    .strings
                                    .get(path.segments[0])
                                    .to_string(),
                            )
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(name) = param_name
                    && let Some(&index) = param_name_to_index.get(&name)
                    && !param_indices.contains(&index)
                {
                    param_indices.push(index);
                }
            }

            if !param_indices.is_empty() {
                return mir::Lifetime::Parameters(param_indices);
            }
        }

        mir::Lifetime::Inferred
    }
}
