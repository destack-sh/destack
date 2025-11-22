use dyst_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};
use dyst_javascript_ast::{Argument, Expression, LocalNodeId, Parameter};

use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a parameter from DIR into JS AST.
    pub fn transpile_parameter(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Parameter>> {
        let parameter = tree.get(parameter_id);
        let parameter = match parameter {
            dir::Parameter::Named {
                modifiers,
                name,
                default,
                symbol,
            } => {
                let symbol = symbols.get_symbol(*symbol);
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let name = unit.strings.intern_from(&module.ast_strings, *name);
                let ty = symbol
                    .declared_ty
                    .map(|ty| self.transpile_type(module, tree, symbols, types, ty, unit))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, symbols, types, default, unit)
                            .expect_node::<Expression>(default.into_global_any(module.id), unit)
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
                symbol: symbol_id,
            } => {
                let symbol = symbols.get_symbol(*symbol_id);
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let pattern =
                    self.transpile_pattern(module, tree, symbols, types, *pattern, unit)?;
                let ty = symbol
                    .declared_ty
                    .map(|ty| self.transpile_type(module, tree, symbols, types, ty, unit))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, symbols, types, default, unit)
                            .expect_node::<Expression>(default.into_global_any(module.id), unit)
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
                symbol: symbol_id,
            } => {
                let symbol = symbols.get_symbol(*symbol_id);
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let name = unit.strings.intern_from(&module.ast_strings, *name);
                let ty = symbol
                    .declared_ty
                    .map(|ty| self.transpile_type(module, tree, symbols, types, ty, unit))
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
        symbols: &SymbolTable,
        types: &TypeTable,
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
                remote_symbol: _,
                value,
            } => {
                let value = self
                    .transpile_expression(module, tree, symbols, types, *value, unit)
                    .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
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
                remote_symbol: _,
                value,
            } => {
                let value = self
                    .transpile_expression(module, tree, symbols, types, *value, unit)
                    .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
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
                remote_symbol: _,
            } => {
                let key = self
                    .transpile_expression(module, tree, symbols, types, *key, unit)
                    .expect_node::<Expression>(key.into_global_any(module.id), unit)?;
                let value = self
                    .transpile_expression(module, tree, symbols, types, *value, unit)
                    .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
                Argument::Dynamic { key, value }
            }
        };
        let argument_id = unit
            .ast
            .insert_from_source(argument, module.id, argument_id);
        Ok(argument_id)
    }
}
