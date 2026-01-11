use destack_dir::{Declarator, Expression, LocalNodeId, Mutability, Pattern};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::GlobalBinding;
use crate::lower::module::ModuleLowerer;

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
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
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
                    node: pattern_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "unsupported module-level binding pattern".to_string(),
                });
            };
            if nested_pattern.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: pattern_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "nested binding patterns not supported for module-level bindings"
                        .to_string(),
                });
            }

            // binding name (module-level bindings must have names)
            let name = self.symbol_name(*symbol_id, pattern_id.into_global_any(self.module_id))?;

            // type (resolve before initializer so we can create properly-typed constants)
            let type_id = self
                .types
                .get_declared_or_inferred_type_id(value_id.into_global_any(self.module_id))
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: value_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "cannot determine type for module-level binding".to_string(),
                })?;
            let mir_type = self.type_lowerer.lower_type(
                self.types,
                type_id,
                self.module_id,
                value_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                &mut self.builder,
            )?;

            // constant initializer
            let initializer = self
                .lower_const_initializer(value_id, mir_type)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: value_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "module-level binding requires constant initializer".to_string(),
                })?;

            // MIR global
            let mir_mutability = lower_mutability(mutability);
            let global_id = self
                .builder
                .global(&name, mir_type, mir_mutability, initializer);
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
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::GlobalInitializer> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => {
                let mir_ty = self.builder.tree().get(mir_type);
                let constant = match (value, mir_ty) {
                    (dir::ScalarLiteral::Boolean(b), mir::Type::Boolean) => {
                        mir::Constant::boolean(*b)
                    }
                    (
                        dir::ScalarLiteral::Integer(i),
                        mir::Type::Int {
                            width,
                            signed: true,
                        },
                    ) => mir::Constant::Int {
                        value: *i,
                        width: *width as u8,
                        is_signed: true,
                    },
                    (
                        dir::ScalarLiteral::Integer(i),
                        mir::Type::Int {
                            width,
                            signed: false,
                        },
                    ) => mir::Constant::UInt {
                        value: *i as u64,
                        width: *width as u8,
                    },
                    (dir::ScalarLiteral::Float(f), mir::Type::Float { width }) => {
                        mir::Constant::Float {
                            bits: if *width == 32 {
                                (*f as f32).to_bits() as u64
                            } else {
                                f.to_bits()
                            },
                            width: *width as u8,
                        }
                    }
                    _ => return None,
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            Expression::Parenthesized { expression } => {
                self.lower_const_initializer(*expression, mir_type)
            }
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
