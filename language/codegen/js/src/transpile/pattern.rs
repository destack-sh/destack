use crate::{
    Expression, LocalNodeId, Pattern, PatternField, TranspileError, TranspileResult,
    TranspileResultExt, Transpiler, TranspilerUnit,
};
use destack_dir as dir;
use destack_dir::{Module, NodeTree, SymbolTable, TypeTable};

impl Transpiler {
    /// Transpile a pattern from DIR into JS AST.
    pub fn transpile_pattern(
        &self,
        module: &Module,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Pattern>> {
        let pattern = tree.get(pattern_id);
        let pattern_id = match pattern {
            dir::Pattern::Wildcard => {
                let name = unit.strings.intern("_");
                let pattern = Pattern::Binding {
                    mutability: None,
                    name,
                };
                unit.ast.insert_from_source(pattern, module.id, pattern_id)
            }
            dir::Pattern::Rest { name } => {
                let name = name.map(|name| unit.strings.intern_from(&self.program.strings, name));
                let pattern = Pattern::Rest { name };
                unit.ast.insert_from_source(pattern, module.id, pattern_id)
            }
            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: pattern_id.into_global_any(module.id),
                    message: None,
                });
            }
        };
        Ok(pattern_id)
    }

    /// Transpile a pattern field from DIR into JS AST.
    pub fn transpile_pattern_field(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<PatternField>> {
        let pattern_field = tree.get(pattern_field_id);
        let pattern_field_id = match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
                symbol: _,
            } => {
                let mutability = mutability.map(|mutability| self.transpile_mutability(mutability));
                let name = unit.strings.intern_from(&self.program.strings, *name);
                let pattern = pattern
                    .map(|pattern| {
                        self.transpile_pattern(module, tree, symbols, types, pattern, unit)
                    })
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, symbols, types, default, unit)
                            .expect_node::<Expression>(default.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                let pattern_field = PatternField::Named {
                    mutability,
                    name,
                    pattern,
                    default,
                };
                unit.ast
                    .insert_from_source(pattern_field, module.id, pattern_field_id)
            }
            dir::PatternField::Alias {
                mutability,
                name,
                alias,
                default,
                symbol: _,
            } => {
                let mutability = mutability.map(|mutability| self.transpile_mutability(mutability));
                let name = unit.strings.intern_from(&self.program.strings, *name);
                let alias = unit.strings.intern_from(&self.program.strings, *alias);
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, tree, symbols, types, default, unit)
                            .expect_node::<Expression>(default.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                let pattern_field = PatternField::Alias {
                    mutability,
                    name,
                    alias,
                    default,
                };
                unit.ast
                    .insert_from_source(pattern_field, module.id, pattern_field_id)
            }
            dir::PatternField::Positional { pattern } => {
                let pattern =
                    self.transpile_pattern(module, tree, symbols, types, *pattern, unit)?;
                let pattern_field = PatternField::Positional { pattern };
                unit.ast
                    .insert_from_source(pattern_field, module.id, pattern_field_id)
            }
        };
        Ok(pattern_field_id)
    }
}
