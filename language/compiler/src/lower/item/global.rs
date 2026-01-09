use destack_dir::{Declarator, Expression, LocalNodeId, Mutability, Pattern};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::{GlobalBinding, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a module-level let/const binding to MIR globals.
    pub(crate) fn lower_module_let(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        mutability: Mutability,
        declarators: &[LocalNodeId<Declarator>],
    ) -> LowerResult<()> {
        for declarator_id in declarators {
            let declarator = self.dir_tree.get(*declarator_id);

            // require an initializer for module-level bindings
            let value_id = declarator
                .value
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: "module-level binding requires initializer".to_string(),
                })?;

            // get the binding pattern
            let pattern_id = declarator.pattern;
            let pattern = self.dir_tree.get(pattern_id);
            let Pattern::Binding {
                symbol: symbol_id,
                pattern: nested_pattern,
                ..
            } = pattern
            else {
                return Err(LowerError::UnsupportedConstruct {
                    node: pattern_id.into_global_any(self.module_id),
                    message: "unsupported module-level binding pattern".to_string(),
                });
            };
            if nested_pattern.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: pattern_id.into_global_any(self.module_id),
                    message: "nested binding patterns not supported for module-level bindings"
                        .to_string(),
                });
            }

            // binding name (module-level bindings must have names)
            let name = self.symbol_name(*symbol_id, pattern_id.into_global_any(self.module_id))?;

            // constant initializer
            let initializer =
                self.lower_const_initializer(value_id)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: value_id.into_global_any(self.module_id),
                        message: "module-level binding requires constant initializer".to_string(),
                    })?;

            // type
            let type_id = self
                .types
                .get_declared_or_inferred_type_id(value_id.into_global_any(self.module_id))
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: value_id.into_global_any(self.module_id),
                    message: "cannot determine type for module-level binding".to_string(),
                })?;
            let mir_type = self.type_lowerer.lower_type(
                self.types,
                type_id,
                self.module_id,
                value_id.into_global_any(self.module_id),
                &mut self.builder,
            )?;

            // MIR global
            let mir_mutability = lower_mutability(mutability);
            let global_id = self.builder.global(&name, mir_type, mir_mutability, initializer);
            let global_symbol_id = symbol_id.into_global(self.module_id);
            self.globals_by_symbol.insert(
                global_symbol_id,
                GlobalBinding {
                    global: global_id,
                    ty: mir_type,
                    mutability: mir_mutability,
                },
            );
        }

        Ok(())
    }

    /// Try to evaluate an expression as a constant initializer.
    pub(crate) fn lower_const_initializer(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<mir::GlobalInitializer> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(b) => {
                    Some(mir::GlobalInitializer::scalar(mir::Constant::boolean(*b)))
                }
                dir::ScalarLiteral::Integer(i) => {
                    Some(mir::GlobalInitializer::scalar(mir::Constant::int64(*i)))
                }
                dir::ScalarLiteral::Float(f) => {
                    Some(mir::GlobalInitializer::scalar(mir::Constant::float64(*f)))
                }
                _ => None,
            },
            Expression::Parenthesized { expression } => self.lower_const_initializer(*expression),
            _ => None,
        }
    }
}

/// Convert DIR mutability to MIR mutability.
pub(crate) fn lower_mutability(mutability: Mutability) -> mir::Mutability {
    match mutability {
        Mutability::Immutable => mir::Mutability::Immutable,
        Mutability::Mutable => mir::Mutability::Mutable,
    }
}
