use crate::{TranspileError, TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};
use dyst_dir as dir;
use dyst_dir::Module;
use dyst_javascript_ast::{Expression, NodeId, Pattern, PatternField};

impl<'a> Transpiler<'a> {
    /// Transpile a pattern from DIR into JS AST.
    pub fn transpile_pattern(
        &self,
        module: &Module,
        pattern_id: dir::NodeId<dir::Pattern>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Pattern>> {
        let pattern = self.session.tree.get(pattern_id);
        let pattern_id = match pattern.as_ref() {
            dir::Pattern::Wildcard => {
                let name = unit.strings.intern("_");
                let pattern = Pattern::Binding {
                    mutability: None,
                    name,
                };
                unit.ast.insert_from_source(pattern, module.id, pattern_id)
            }
            dir::Pattern::Rest { name } => {
                let name = name.map(|name| unit.strings.intern_from(&self.session.strings, name));
                let pattern = Pattern::Rest { name };
                unit.ast.insert_from_source(pattern, module.id, pattern_id)
            }
            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: pattern_id.into_any(),
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
        pattern_field_id: dir::NodeId<dir::PatternField>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<PatternField>> {
        let pattern_field = self.session.tree.get(pattern_field_id);
        let pattern_field_id = match pattern_field.as_ref() {
            dir::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.transpile_mutability(mutability));
                let name = unit.strings.intern_from(&self.session.strings, *name);
                let pattern = pattern
                    .map(|pattern| self.transpile_pattern(module, pattern, unit))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, default, unit)
                            .expect_node::<Expression>(default.into_any(), unit)
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
            } => {
                let mutability = mutability.map(|mutability| self.transpile_mutability(mutability));
                let name = unit.strings.intern_from(&self.session.strings, *name);
                let alias = unit.strings.intern_from(&self.session.strings, *alias);
                let default = default
                    .map(|default| {
                        self.transpile_expression(module, default, unit)
                            .expect_node::<Expression>(default.into_any(), unit)
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
                let pattern = self.transpile_pattern(module, *pattern, unit)?;
                let pattern_field = PatternField::Positional { pattern };
                unit.ast
                    .insert_from_source(pattern_field, module.id, pattern_field_id)
            }
        };
        Ok(pattern_field_id)
    }
}
