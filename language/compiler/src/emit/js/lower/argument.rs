use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a parameter from DIR into JavaScript.
    pub(crate) fn lower_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> Result<js::LocalNodeId<js::Parameter>, EmitError> {
        let source_parameter_id = parameter_id;
        let parameter = self.dir_tree.get(parameter_id);
        match parameter {
            dir::Parameter::Named { name, default, .. } => {
                let name = *name;
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
                    .transpose()?;
                let parameter = js::Parameter::Named { name, default };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::Pattern {
                pattern, default, ..
            } => {
                let pattern = self.lower_pattern(*pattern)?;
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
                    .transpose()?;
                let parameter = js::Parameter::Pattern { pattern, default };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = *name;
                let parameter = js::Parameter::VariadicNamed { name };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::VariadicPattern { pattern, .. } => {
                let pattern = self.lower_pattern(*pattern)?;
                let parameter = js::Parameter::VariadicPattern { pattern };
                let parameter_id =
                    self.tree
                        .insert_from_source(parameter, self.module.id, parameter_id);
                self.copy_source_node_symbol(parameter_id, source_parameter_id);
                Ok(parameter_id)
            }
            dir::Parameter::Error => Err(self.unhandled(
                parameter_id.into_global_any(self.module.id),
                Some("parameter error slots are not lowered to JS".to_string()),
            )),
        }
    }

    /// Lower an argument from DIR into JavaScript.
    pub(crate) fn lower_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> Result<js::LocalNodeId<js::Argument>, EmitError> {
        let argument = self.dir_tree.get(argument_id);
        let argument = match argument {
            dir::Argument::Positional { value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                js::Argument::Positional { value }
            }
            dir::Argument::Spread { value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                js::Argument::Spread { value }
            }
            dir::Argument::Elision => {
                return Err(self.unhandled(
                    argument_id.into_global_any(self.module.id),
                    Some("array elisions are only lowered in array literals".to_string()),
                ));
            }
            dir::Argument::Error => {
                return Err(self.unhandled(
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
