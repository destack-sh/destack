use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, LifetimeParameters, ModuleLowerer, TypeSubstitution};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Declare one MIR global for each binding of one module-level let.
    pub(in crate::lower) fn declare_module_constants(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        if mutability != dir::Mutability::Immutable {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a mutable module binding".to_string(),
            }
            .into());
        }

        for declarator in declarators {
            let pattern = self.local().tree().get(*declarator).pattern;
            let node = pattern.into_global_any(self.module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a destructured module binding".to_string(),
                }
                .into());
            };
            let Some(term) = self.module_constant(symbol)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a module binding with a runtime initializer".to_string(),
                }
                .into());
            };

            // declare the constant under its module-qualified name
            let declared = self.symbol_type(symbol)?;
            let initializer = self.constant_initializer(builder, &term, declared)?;
            let ty = self.constant_type(builder, declared)?;
            let name = self.constant_name(symbol)?;
            let global = builder.global_constant(&name, ty, initializer);
            self.globals.insert(symbol, global);
        }

        Ok(())
    }

    /// Return the evaluated constant behind one module binding, when one exists.
    pub(in crate::lower) fn module_constant(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let statics = &self.state(symbol.module_id)?.statics;
        let Some(id) = statics.get_symbol_static_id(symbol) else {
            return Ok(None);
        };

        Ok(statics.get_static_maybe(id.local_id).cloned())
    }

    /// Return the module-qualified name of one constant binding.
    pub(in crate::lower) fn constant_name(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        let Some(name) = self.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a module constant without a name".to_string(),
            });
        };
        let path = &self.state(symbol.module_id)?.path;

        Ok(format!("{path}.{}", self.strings.get(name)))
    }

    /// Build the initializer of one constant from its evaluated term.
    fn constant_initializer(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        term: &dir::StaticTerm,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::GlobalInitializer> {
        match term {
            // scalars initialize at their lowered carrier
            dir::StaticTerm::ScalarLiteral { value } => {
                let pointer_bytes = builder.pointer_bytes();
                let carrier = self.constant_type(builder, ty)?;
                let carrier = builder.tree_mut().get(carrier).clone();
                let constant = Self::scalar_constant(*value, &carrier, pointer_bytes)?;

                Ok(mir::GlobalInitializer::Scalar(constant))
            }

            // newtypes wrap their backing as a single-field aggregate
            dir::StaticTerm::Newtype {
                ty: instance,
                value,
            } => {
                let dir::Type::Application(application) = self.ty(*instance)? else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR built a newtype constant without its instance"
                            .to_string(),
                    });
                };
                let Some(dir::Definition::Newtype(newtype)) =
                    self.definition(application.symbol)?
                else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR built a newtype constant without its definition"
                            .to_string(),
                    });
                };
                let backing = newtype.backing;
                let inner = self.constant_initializer(builder, value, backing)?;

                Ok(mir::GlobalInitializer::Aggregate(vec![inner]))
            }

            // tuples initialize each element at its own type
            dir::StaticTerm::Tuple { elements } => {
                let reduced = self.reduced_type(ty)?;
                let dir::Type::Tuple(tuple) = self.ty(reduced)? else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR built a tuple constant at a non-tuple type"
                            .to_string(),
                    });
                };
                let element_types = self
                    .types(reduced.module_id)?
                    .type_ids(tuple.elements)
                    .to_vec();
                if element_types.len() != elements.len() {
                    return Err(CompilerError::Internal {
                        message: "checked DIR built a tuple constant with a mismatched arity"
                            .to_string(),
                    });
                }
                let mut values = Vec::with_capacity(elements.len());
                for (element, element_type) in elements.iter().zip(element_types) {
                    values.push(self.constant_initializer(builder, element, element_type)?);
                }

                Ok(mir::GlobalInitializer::Aggregate(values))
            }

            _ => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a composite module constant".to_string(),
            }
            .into()),
        }
    }

    /// Lower one constant's checked type outside any generic context.
    fn constant_type(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let pointer_bytes = builder.pointer_bytes();
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();

        self.type_lowerer(
            builder.tree_mut(),
            pointer_bytes,
            &type_substitution,
            &lifetime_parameters,
        )
        .lower(ty)
    }

    /// Build one scalar constant at its lowered carrier.
    fn scalar_constant(
        literal: dir::ScalarLiteral,
        carrier: &mir::Type,
        pointer_bytes: u8,
    ) -> CompilerResult<mir::Constant> {
        Ok(match (literal, carrier) {
            (dir::ScalarLiteral::Boolean(value), _) => mir::Constant::Boolean { value },
            (
                dir::ScalarLiteral::Integer(value),
                mir::Type::Int {
                    width,
                    is_signed: true,
                },
            ) => mir::Constant::Int {
                value: value as i128,
                width: *width,
                is_signed: true,
            },
            (
                dir::ScalarLiteral::Integer(value),
                mir::Type::Int {
                    width,
                    is_signed: false,
                },
            ) => mir::Constant::UInt {
                value: value as u128,
                width: *width,
            },
            (dir::ScalarLiteral::Integer(value), mir::Type::Usize) => mir::Constant::UInt {
                value: value as u128,
                width: 8 * pointer_bytes as u16,
            },
            (dir::ScalarLiteral::Integer(value), mir::Type::Isize) => mir::Constant::Int {
                value: value as i128,
                width: 8 * pointer_bytes as u16,
                is_signed: true,
            },
            (dir::ScalarLiteral::Integer(value), mir::Type::Float(format)) => {
                mir::Constant::Float {
                    bits: destack_core::float_to_bits(format.format(), value as f64),
                    format: *format,
                }
            }
            (dir::ScalarLiteral::Float(value), mir::Type::Float(format)) => mir::Constant::Float {
                bits: destack_core::float_to_bits(format.format(), value),
                format: *format,
            },
            (literal, carrier) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "checked DIR selected carrier {carrier:?} for constant {literal:?}"
                    ),
                });
            }
        })
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the global behind one module constant, importing foreign ones.
    pub(in crate::lower) fn module_constant_global(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Global>>> {
        // reuse the global this module already declared or imported
        if let Some(global) = self.lowerer.globals.get(&symbol) {
            return Ok(Some(*global));
        }

        // only foreign constants import lazily; local ones declare at roots
        if symbol.module_id == self.lowerer.module
            || self.lowerer.module_constant(symbol)?.is_none()
        {
            return Ok(None);
        }

        // declare an import for one foreign module constant
        let ty = self.lowerer.symbol_type(symbol)?;
        let ty = self.lower_type(ty)?;
        let name = self.lowerer.constant_name(symbol)?;
        let global = self
            .builder
            .external_global(&name, ty, mir::Mutability::Immutable);
        self.lowerer.globals.insert(symbol, global);

        Ok(Some(global))
    }
}
