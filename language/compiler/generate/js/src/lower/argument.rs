use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Build one JS binding modifier when any field is present.
    fn build_binding_modifier(
        &self,
        kind: Option<js::BindingKind>,
        variance: Option<js::VarianceModifier>,
        anchor: Option<js::BindingAnchor>,
        mutability: Option<js::Mutability>,
        visibility: Option<js::Visibility>,
        operator: Option<js::BindingOperator>,
        accessor: Option<js::AccessorKind>,
    ) -> Option<js::BindingModifier> {
        let modifiers = js::BindingModifier {
            kind,
            variance,
            anchor,
            mutability,
            visibility,
            operator,
            definite: false,
            accessor,
        };

        if modifiers == js::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        }
    }

    /// Lower a parameter from DIR into JS AST.
    pub fn lower_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Parameter>> {
        let source_parameter_id = parameter_id;
        let parameter = self.dir_tree.get(parameter_id);
        match parameter {
            dir::Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                declared_type: _,
                default,
            } => {
                let modifiers = self.build_binding_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    None,
                    None,
                    if *is_readonly {
                        Some(js::Mutability::Immutable)
                    } else {
                        None
                    },
                    visibility.map(|visibility| self.lower_visibility(visibility)),
                    None,
                    None,
                );
                let name = *name;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
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
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::Pattern {
                pattern,
                is_optional,
                declared_type: _,
                default,
            } => {
                let modifiers = self.build_binding_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
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
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                declared_type: _,
            } => {
                let modifiers = self.build_binding_modifier(
                    None,
                    None,
                    None,
                    if *is_readonly {
                        Some(js::Mutability::Immutable)
                    } else {
                        None
                    },
                    visibility.map(|visibility| self.lower_visibility(visibility)),
                    None,
                    None,
                );
                let name = *name;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
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
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type: _,
            } => {
                let modifiers = None;
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
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
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::Error => Err(CodegenJsError::UnsupportedConstruct {
                node: parameter_id.into_global_any(self.module.id),
                message: Some("parameter error slots are not lowered to JS".to_string()),
            }),
        }
    }

    /// Lower an argument from DIR into JS AST.
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
            dir::Argument::Error => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: argument_id.into_global_any(self.module.id),
                    message: Some("argument error slots are not lowered to JS".to_string()),
                });
            }
        };
        let argument_id = self
            .tree
            .insert_from_source(argument, self.module.id, argument_id);
        Ok(argument_id)
    }
}
