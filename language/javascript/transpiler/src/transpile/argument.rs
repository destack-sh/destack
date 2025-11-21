use dyst_dir::{self as dir, Module, NodeTree};
use dyst_javascript_ast::{Argument, Expression, LocalNodeId, Parameter};

use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a parameter from DIR into JS AST.
    pub fn transpile_parameter(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Parameter>> {
        let parameter = tree.get(parameter_id);
        let parameter = match parameter {
            dir::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let name = unit.strings.intern_from(&module.ast_strings, *name);
                let ty = ty
                    .map(|ty| self.transpile_type(module, tree, ty, unit))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, default, unit)
                            .expect_node::<Expression>(default.into_any(), unit)
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
                ty,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let pattern = self.transpile_pattern(module, tree, *pattern, unit)?;
                let ty = ty
                    .map(|ty| self.transpile_type(module, tree, ty, unit))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, default, unit)
                            .expect_node::<Expression>(default.into_any(), unit)
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
                ty,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let name = unit.strings.intern_from(&module.ast_strings, *name);
                let ty = ty
                    .map(|ty| self.transpile_type(module, tree, ty, unit))
                    .transpose()?;
                Parameter::Variadic {
                    modifiers,
                    name,
                    ty,
                }
            }
        };
        let parameter_id = unit
            .ast
            .insert_from_source(parameter, module.id, parameter_id);
        Ok(parameter_id)
    }

    /// Transpile a argument from DIR into JS AST.
    pub fn transpile_argument(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        argument_id: dir::LocalNodeId<dir::Argument>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Argument>> {
        let argument = tree.get(argument_id);
        let argument = match argument {
            dir::Argument::UnresolvedNamed {
                modifiers: _,
                name: _,
                value,
            }
            | dir::Argument::UnresolvedPositional {
                modifiers: _,
                value,
            }
            | dir::Argument::Direct {
                modifiers: _,
                name: _,
                symbol,
                value,
            } => {
                let value = self
                    .transpile_expression(module, tree, *value, unit)
                    .expect_node::<Expression>(value.into_any(), unit)?;
                Argument::Positional { value }
            }
            dir::Argument::UnresolvedSpread {
                modifiers: _,
                name: _,
                value,
            }
            | dir::Argument::Spread {
                modifiers: _,
                name: _,
                symbol,
                value,
            } => {
                let value = self
                    .transpile_expression(module, tree, *value, unit)
                    .expect_node::<Expression>(value.into_any(), unit)?;
                Argument::Spread { value }
            }
            dir::Argument::UnresolvedDynamic {
                modifiers: _,
                name: _,
                key,
                value,
            }
            | dir::Argument::Dynamic {
                modifiers: _,
                name: _,
                key,
                value,
                symbol,
            } => {
                let key = self
                    .transpile_expression(module, tree, *key, unit)
                    .expect_node::<Expression>(key.into_any(), unit)?;
                let value = self
                    .transpile_expression(module, tree, *value, unit)
                    .expect_node::<Expression>(value.into_any(), unit)?;
                Argument::Dynamic { key, value }
            }
        };
        let argument_id = unit
            .ast
            .insert_from_source(argument, module.id, argument_id);
        Ok(argument_id)
    }
}
