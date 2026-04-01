use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a parameter from DIR into JS AST.
    pub fn lower_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Parameter>> {
        let parameter = self.dir_tree.get(parameter_id);
        match parameter {
            dir::Parameter::Named {
                modifiers,
                name,
                default,
                symbol,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let name = self.strings.intern_from(self.source_strings, *name);
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default)
                            .expect_node::<js::Expression>(
                                default.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;
                let parameter = js::Parameter::Named {
                    modifiers,
                    name,
                    ty,
                    default,
                };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.set_source_node_symbol(parameter_id, *symbol);
                Ok(parameter_id)
            }
            dir::Parameter::Pattern {
                modifiers,
                pattern,
                default,
                symbol,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default)
                            .expect_node::<js::Expression>(
                                default.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;
                let parameter = js::Parameter::Pattern {
                    modifiers,
                    pattern,
                    ty,
                    default,
                };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.set_source_node_symbol(parameter_id, *symbol);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicNamed {
                modifiers,
                name,
                symbol,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let name = self.strings.intern_from(self.source_strings, *name);
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let parameter = js::Parameter::VariadicNamed {
                    modifiers,
                    name,
                    ty,
                };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.set_source_node_symbol(parameter_id, *symbol);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicPattern {
                modifiers,
                pattern,
                symbol,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let parameter = js::Parameter::VariadicPattern {
                    modifiers,
                    pattern,
                    ty,
                };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.set_source_node_symbol(parameter_id, *symbol);
                Ok(parameter_id)
            }
            dir::Parameter::Error { .. } => Err(CodegenJsError::UnsupportedConstruct {
                node: parameter_id.into_global_any(self.module.id),
                message: Some("parameter error slots are not lowered to js".to_string()),
            }),
        }
    }

    /// Lower a argument from DIR into JS AST.
    pub fn lower_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Argument>> {
        let argument = self.dir_tree.get(argument_id);
        let argument = match argument {
            dir::Argument::Named { name: _, value, .. }
            | dir::Argument::Labeled {
                label: _, value, ..
            }
            | dir::Argument::Positional { value, .. } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                js::Argument::Positional { value }
            }
            dir::Argument::Spread {
                label: _, value, ..
            } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                js::Argument::Spread { value }
            }
            dir::Argument::Error { value } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: value.into_global_any(self.module.id),
                    message: Some("argument error slots are not lowered to js".to_string()),
                });
            }
        };
        let argument_id = self
            .tree
            .insert_from_source(argument, self.module.id, argument_id);
        Ok(argument_id)
    }
}
