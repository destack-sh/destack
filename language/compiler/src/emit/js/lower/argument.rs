use crate::EmitError;
use destack_dir as dir;
use destack_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a parameter from DIR into JS AST.
    pub(crate) fn lower_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> Result<js::LocalNodeId<js::Parameter>, EmitError> {
        let source_parameter_id = parameter_id;
        let parameter = self.dir_tree.get(parameter_id);
        match parameter {
            dir::Parameter::Named {
                name,
                is_optional,
                declared_type: _,
                default,
                ..
            } => {
                let modifiers = is_optional.then_some(js::BindingModifier {
                    kind: Some(js::BindingKind::Maybe),
                    ..js::BindingModifier::default()
                });
                let name = *name;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty, parameter_id.into_any()))
                    .transpose()?;
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
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
                ..
            } => {
                let modifiers = is_optional.then_some(js::BindingModifier {
                    kind: Some(js::BindingKind::Maybe),
                    ..js::BindingModifier::default()
                });
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty, parameter_id.into_any()))
                    .transpose()?;
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
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
            dir::Parameter::VariadicNamed { name, .. } => {
                let modifiers = None;
                let name = *name;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty, parameter_id.into_any()))
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
            dir::Parameter::VariadicPattern { pattern, .. } => {
                let modifiers = None;
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_node_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty, parameter_id.into_any()))
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
            dir::Parameter::Error => Err(self.unsupported_construct(
                parameter_id.into_global_any(self.module.id),
                Some("parameter error slots are not lowered to JS".to_string()),
            )),
        }
    }

    /// Lower an argument from DIR into JS AST.
    pub(crate) fn lower_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> Result<js::LocalNodeId<js::Argument>, EmitError> {
        let argument = self.dir_tree.get(argument_id);
        let argument = match argument {
            dir::Argument::Named { name: _, value, .. }
            | dir::Argument::Labeled {
                label: _, value, ..
            }
            | dir::Argument::Positional { value, .. } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                js::Argument::Positional { value }
            }
            dir::Argument::Spread {
                label: _, value, ..
            } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                js::Argument::Spread { value }
            }
            dir::Argument::Error => {
                return Err(self.unsupported_construct(
                    argument_id.into_global_any(self.module.id),
                    Some("argument error slots are not lowered to JS".to_string()),
                ));
            }
        };
        let argument_id = self
            .tree
            .insert_from_source(argument, self.module.id, argument_id);
        Ok(argument_id)
    }
}
