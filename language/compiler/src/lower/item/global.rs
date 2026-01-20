use destack_dir::{
    Declarator, EnumFieldValue, Expression, LocalNodeId, Mutability, Pattern, Resolution,
};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::GlobalBinding;
use crate::lower::module::ModuleLowerer;
use crate::lower::r#type::{EnumFieldValueDescriptor, enum_field_value_for_symbol};

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
            let mir_type = self.lower_type(
                type_id,
                value_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
            )?;

            // constant initializer
            let initializer = self
                .lower_const_initializer(value_id, mir_type)?
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: value_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "module-level binding requires constant initializer".to_string(),
                })?;

            // mir global
            let mir_mutability = lower_mutability(mutability);
            let global_id = self
                .builder
                .global(&name, mir_type, mir_mutability, initializer);
            let global_symbol_id = symbol_id.into_global(self.module_id);
            let binding = GlobalBinding {
                global: global_id,
                ty: mir_type,
                mutability: mir_mutability,
            };
            self.insert_global_binding(global_symbol_id, binding)?;
        }

        Ok(())
    }

    /// Try to evaluate an expression as a constant initializer.
    pub(crate) fn lower_const_initializer(
        &self,
        expression_id: LocalNodeId<Expression>,
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<Option<mir::GlobalInitializer>> {
        let expression = self.dir_tree.get(expression_id);
        let initializer = match expression {
            Expression::ScalarLiteral { value } => self.lower_const_scalar_literal(value, mir_type),
            Expression::Parenthesized { expression } => {
                return self.lower_const_initializer(*expression, mir_type);
            }
            Expression::Cast { value, .. } => {
                return self.lower_const_initializer(*value, mir_type);
            }
            Expression::Member { .. } => {
                return self.lower_const_member_initializer(expression_id, mir_type);
            }
            _ => None,
        };

        Ok(initializer)
    }

    /// Lower scalar literal constants into global initializers.
    fn lower_const_scalar_literal(
        &self,
        value: &dir::ScalarLiteral,
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::GlobalInitializer> {
        let mir_ty = self.builder.tree().get(mir_type);
        let constant = match (value, mir_ty) {
            (dir::ScalarLiteral::Boolean(b), mir::Type::Boolean) => mir::Constant::boolean(*b),
            (
                dir::ScalarLiteral::Integer(i),
                mir::Type::Int {
                    width,
                    is_signed: true,
                },
            ) => mir::Constant::Int {
                value: *i,
                width: *width as u8,
                is_signed: true,
            },
            (dir::ScalarLiteral::Integer(i), mir::Type::Isize) => {
                let width = self.type_lowerer.pointer_width_bits();
                mir::Constant::Int {
                    value: *i,
                    width: width as u8,
                    is_signed: true,
                }
            }
            (
                dir::ScalarLiteral::Integer(i),
                mir::Type::Int {
                    width,
                    is_signed: false,
                },
            ) => mir::Constant::UInt {
                value: *i as u64,
                width: *width as u8,
            },
            (dir::ScalarLiteral::Integer(i), mir::Type::Usize) => {
                let width = self.type_lowerer.pointer_width_bits();
                mir::Constant::UInt {
                    value: *i as u64,
                    width: width as u8,
                }
            }
            (dir::ScalarLiteral::Float(f), mir::Type::Float { width }) => mir::Constant::Float {
                bits: if *width == 32 {
                    (*f as f32).to_bits() as u64
                } else {
                    f.to_bits()
                },
                width: *width as u8,
            },
            _ => return None,
        };

        Some(mir::GlobalInitializer::scalar(constant))
    }

    /// Lower static member constants into global initializers.
    fn lower_const_member_initializer(
        &self,
        expression_id: LocalNodeId<Expression>,
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<Option<mir::GlobalInitializer>> {
        // resolve the static symbol target for the member expression
        let node_id = expression_id.into_global_any(self.module_id);
        let resolution_id = self.types.get_resolution_for_node(node_id);
        let Some(resolution_id) = resolution_id else {
            return Ok(None);
        };
        let resolution = self.types.get_resolution(resolution_id);
        let Resolution::Static { candidate, .. } = resolution else {
            return Ok(None);
        };

        // resolve enum field constants when the target is an enum field
        let node = node_id.into_anchored(Some(self.profile));
        let Some(EnumFieldValueDescriptor { backing: _, value }) = enum_field_value_for_symbol(
            &self.compiler.program,
            self.profile,
            candidate.target_symbol,
            node,
        )?
        else {
            return Ok(None);
        };

        // build the enum constant initializer
        let mir_ty = self.builder.tree().get(mir_type);
        let initializer = match (value, mir_ty) {
            (
                EnumFieldValue::Int(value),
                mir::Type::Int {
                    width,
                    is_signed: signed,
                },
            ) => {
                let constant = if *signed {
                    mir::Constant::Int {
                        value,
                        width: *width as u8,
                        is_signed: true,
                    }
                } else {
                    mir::Constant::UInt {
                        value: value as u64,
                        width: *width as u8,
                    }
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (EnumFieldValue::Int(value), mir::Type::Isize) => {
                let width = self.type_lowerer.pointer_width_bits();
                let constant = mir::Constant::Int {
                    value,
                    width: width as u8,
                    is_signed: true,
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (EnumFieldValue::Int(value), mir::Type::Usize) => {
                let width = self.type_lowerer.pointer_width_bits();
                let constant = mir::Constant::UInt {
                    value: value as u64,
                    width: width as u8,
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (EnumFieldValue::String(value), _) => {
                let string_type = self.type_lowerer.string_type();
                if string_type != Some(mir_type) {
                    return Ok(None);
                }
                let literal = self.compiler.program.strings.get(value).to_string();
                let constant = mir::Constant::String { value: literal };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            _ => None,
        };

        Ok(initializer)
    }
}

/// Convert DIR mutability to MIR mutability.
pub(crate) fn lower_mutability(mutability: Mutability) -> mir::Mutability {
    match mutability {
        Mutability::Immutable => mir::Mutability::Immutable,
        Mutability::Mutable => mir::Mutability::Mutable,
    }
}
