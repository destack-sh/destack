use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError};

use crate::lower::r#type::{EnumFieldValueDescriptor, enum_field_value_for_symbol};
use crate::lower::{GlobalBinding, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a module-level let/const binding to MIR globals.
    pub(crate) fn lower_module_let(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            let declarator = self.dir_tree.get(*declarator_id);

            // require an initializer for module-level bindings
            let value_id = declarator
                .value
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "module-level binding requires initializer".to_string(),
                })
                .map_err(CompilerError::from)?;

            // get the binding pattern
            let pattern_id = declarator.pattern;
            let pattern = self.dir_tree.get(pattern_id);
            let dir::Pattern::Binding {
                symbol: symbol_id,
                pattern: nested_pattern,
                ..
            } = pattern
            else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        pattern_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "unsupported module-level binding pattern".to_string(),
                }
                .into());
            };
            if nested_pattern.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        pattern_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "nested binding patterns not supported for module-level bindings"
                        .to_string(),
                }
                .into());
            }

            // binding name (module-level bindings must have names)
            let name = self.symbol_name(*symbol_id, pattern_id.into_global_any(self.module_id))?;

            // type (resolve before initializer so we can create properly-typed constants)
            let type_id = self
                .types
                .get_declared_or_inferred_type_id(value_id.into_global_any(self.module_id))
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        value_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "cannot determine type for module-level binding".to_string(),
                })
                .map_err(CompilerError::from)?;
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
                    anchor: self.diagnostic_anchor(
                        value_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "module-level binding requires constant initializer".to_string(),
                })
                .map_err(CompilerError::from)?;

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
                space: self.builder.tree().get(global_id).space.clone(),
            };
            self.insert_global_binding(global_symbol_id, binding)?;
        }

        Ok(())
    }

    /// Try to evaluate an expression as a constant initializer.
    pub(crate) fn lower_const_initializer(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<mir::GlobalInitializer>> {
        let expression = self.dir_tree.get(expression_id);
        let initializer = match expression {
            dir::Expression::ScalarLiteral { value } => {
                self.lower_const_scalar_literal(value, mir_type)
            }
            dir::Expression::Parenthesized { expression } => {
                return self.lower_const_initializer(*expression, mir_type);
            }
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => {
                return self.lower_const_initializer(*expression, mir_type);
            }
            dir::Expression::Member { .. } | dir::Expression::PrivateMember { .. } => {
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
        // peel transparent nominal wrappers before matching initializer payloads
        let Ok(mir_type) = self.global_initializer_repr_type(mir_type) else {
            return None;
        };
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
                value: i128::from(*i),
                width: *width,
                is_signed: true,
            },
            (dir::ScalarLiteral::Integer(i), mir::Type::Isize) => {
                let width = self.type_lowerer.pointer_width_bits();
                mir::Constant::Int {
                    value: i128::from(*i),
                    width,
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
                value: *i as u128,
                width: *width,
            },
            (dir::ScalarLiteral::Integer(i), mir::Type::Usize) => {
                let width = self.type_lowerer.pointer_width_bits();
                mir::Constant::UInt {
                    value: *i as u128,
                    width,
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

    /// Resolve the physical representation type for one global initializer.
    fn global_initializer_repr_type(
        &self,
        mut mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        loop {
            let mir::Type::Newtype { inner, .. } = self.builder.tree().get(mir_type) else {
                return Ok(mir_type);
            };

            mir_type = inner
                .ty()
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        dir::GlobalNodeIdAny::new(self.module_id, self.module_node)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "global initializer newtype inner type is not concrete".to_string(),
                })
                .map_err(CompilerError::from)?;
        }
    }

    /// Lower static member constants into global initializers.
    fn lower_const_member_initializer(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<mir::GlobalInitializer>> {
        // resolve the static symbol target for the member expression
        let node_id = expression_id.into_global_any(self.module_id);
        let Some(resolution) = self.types.resolution(node_id) else {
            return Ok(None);
        };
        let dir::Resolution::Dispatch(dir::DispatchResolution::Static { target, .. }) = resolution
        else {
            return Ok(None);
        };

        // resolve enum field constants when the target is an enum field
        let node = node_id.into_anchored(Some(self.profile));
        let anchor = self.diagnostic_anchor(node);
        let Some(EnumFieldValueDescriptor { backing: _, value }) = enum_field_value_for_symbol(
            self.compiler,
            self.context,
            self.profile,
            target.symbol,
            anchor,
        )?
        else {
            return Ok(None);
        };

        // build the enum constant initializer
        let mir_type = self.global_initializer_repr_type(mir_type)?;
        let mir_ty = self.builder.tree().get(mir_type);
        let initializer = match (value, mir_ty) {
            (
                dir::EnumFieldValue::Int(value),
                mir::Type::Int {
                    width,
                    is_signed: signed,
                },
            ) => {
                let constant = if *signed {
                    mir::Constant::Int {
                        value: i128::from(value),
                        width: *width,
                        is_signed: true,
                    }
                } else {
                    mir::Constant::UInt {
                        value: value as u128,
                        width: *width,
                    }
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (dir::EnumFieldValue::Int(value), mir::Type::Isize) => {
                let width = self.type_lowerer.pointer_width_bits();
                let constant = mir::Constant::Int {
                    value: i128::from(value),
                    width,
                    is_signed: true,
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (dir::EnumFieldValue::Int(value), mir::Type::Usize) => {
                let width = self.type_lowerer.pointer_width_bits();
                let constant = mir::Constant::UInt {
                    value: value as u128,
                    width,
                };
                Some(mir::GlobalInitializer::scalar(constant))
            }
            (dir::EnumFieldValue::String(value), _) => {
                let string_type = self.type_lowerer.string_type();
                if string_type != Some(mir_type) {
                    return Ok(None);
                }
                let literal = self.strings.get(value);
                Some(mir::GlobalInitializer::string(literal.as_ref()))
            }
            _ => None,
        };

        Ok(initializer)
    }
}

/// Convert DIR mutability to MIR mutability.
pub(crate) fn lower_mutability(mutability: dir::Mutability) -> mir::Mutability {
    match mutability {
        dir::Mutability::Immutable => mir::Mutability::Immutable,
        dir::Mutability::Mutable => mir::Mutability::Mutable,
    }
}
