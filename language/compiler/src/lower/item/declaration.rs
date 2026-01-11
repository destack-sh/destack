use destack_dir::{Declaration, DynamicKey, LocalNodeId, Member};

use crate::{LocalBinding, LowerError, LowerResult, Terminates};

use super::super::ModuleLowerer;

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

    /// Lower a method member to a MIR function.
    fn lower_method(
        &mut self,
        member_id: LocalNodeId<Member>,
        member: &Member,
        this_type: Option<destack_mir::LocalNodeId<destack_mir::Type>>,
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
            let parameter_node =
                destack_dir::GlobalNodeId::new(self.module_id, *parameter_id).into();
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
        let mut function_lowerer = super::super::block::FunctionLowerer::new(
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
        let entry_block = function_lowerer.builder.create_block();
        function_lowerer.builder.switch_to_block(entry_block);

        // add this parameter as first local (if present)
        let mut param_index = 0;
        if let Some(this_ty) = this_type {
            let this_variable = function_lowerer.builder.create_variable(this_ty);
            let this_value = function_lowerer.builder.function_parameter(param_index);
            function_lowerer
                .builder
                .define_variable(this_variable, this_value);

            // set this_binding for Expression::This lookup
            function_lowerer.this_binding = Some(LocalBinding {
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
            let variable = function_lowerer.builder.create_variable(ty);
            let value = function_lowerer.builder.function_parameter(param_index);
            function_lowerer.builder.define_variable(variable, value);
            function_lowerer
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });
            param_index += 1;
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_lowerer.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_lowerer.builder.return_(None);
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
            function_lowerer.builder.return_(None);
        }

        function_lowerer.builder.finish();
        Ok(())
    }

    /// Resolve a method return type for lowering.
    fn resolve_method_return_type(
        &mut self,
        member_id: LocalNodeId<Member>,
        member_node: destack_dir::GlobalNodeIdAny,
    ) -> LowerResult<destack_mir::LocalNodeId<destack_mir::Type>> {
        // get signature type from inferred types
        let signature_type_id = self
            .types
            .get_inferred_type_id(member_node)
            .ok_or_else(|| LowerError::MissingType {
                node: member_node.into_anchored(Some(self.profile)),
            })?;

        // extract return type from function signature
        let destack_dir::Type::Function { return_type, .. } =
            self.types.get_type(signature_type_id)
        else {
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
