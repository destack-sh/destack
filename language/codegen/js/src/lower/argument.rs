use crate::{
    Argument, CodegenJsResult, CodegenJsResultExt, Expression, LocalNodeId, ModuleLowerer,
    Parameter,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a parameter from DIR into JS AST.
    pub fn lower_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> CodegenJsResult<LocalNodeId<Parameter>> {
        let parameter = self.dir_tree.get(parameter_id);
        let parameter = match parameter {
            dir::Parameter::Named {
                modifiers,
                name,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let name = self.strings.intern_from(&self.module.ast.strings, *name);
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Parameter::Named {
                    modifiers,
                    name,
                    ty,
                    default,
                }
            }
            dir::Parameter::Pattern {
                modifiers,
                pattern,
                default,
                symbol: _,
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
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Parameter::Pattern {
                    modifiers,
                    pattern,
                    ty,
                    default,
                }
            }
            dir::Parameter::Variadic {
                modifiers,
                name,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let name = self.strings.intern_from(&self.module.ast.strings, *name);
                let ty = self
                    .types
                    .get_declared_type_id(parameter_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                Parameter::Variadic {
                    modifiers,
                    name,
                    ty,
                }
            }
        };
        let parameter_id = self
            .tree
            .insert_from_source(parameter, self.module.id, parameter_id);
        Ok(parameter_id)
    }

    /// Lower a argument from DIR into JS AST.
    pub fn lower_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> CodegenJsResult<LocalNodeId<Argument>> {
        let argument = self.dir_tree.get(argument_id);
        let argument = match argument {
            dir::Argument::Named { name: _, value }
            | dir::Argument::Labeled { label: _, value }
            | dir::Argument::Positional { value } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                Argument::Positional { value }
            }
            dir::Argument::Spread { value } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                Argument::Spread { value }
            }
            dir::Argument::Dynamic { key, value } => {
                let key = self
                    .lower_expression(*key)
                    .expect_node::<Expression>(key.into_global_any(self.module.id), self)?;
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                Argument::Dynamic { key, value }
            }
        };
        let argument_id = self
            .tree
            .insert_from_source(argument, self.module.id, argument_id);
        Ok(argument_id)
    }
}
