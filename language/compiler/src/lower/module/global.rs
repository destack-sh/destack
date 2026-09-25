use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{FunctionLowerer, GenericScope, ModuleInitializer, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Declare one global for each binding of one module-level let.
    pub(in crate::lower) fn declare_module_constants(
        &mut self,
        tree: &mut mir::Tree,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        // require an immutable module binding
        if mutability != dir::Mutability::Immutable {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a mutable module binding".to_string(),
            }
            .into());
        }

        // declare one global per bound name, in declaration order
        for declarator in declarators {
            // resolve the symbol the declarator binds
            let pattern = self.local().tree().get(*declarator).pattern;
            let node = pattern.into_global_any(self.module);
            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a destructured module binding".to_string(),
                }
                .into());
            };
            let value = self.local().tree().get(*declarator).value;
            self.declare_constant(tree, symbol, value)?;
        }

        Ok(())
    }

    /// Declare the constant global behind one associated const, its value static by sema's rule.
    pub(in crate::lower) fn declare_associated_const(
        &mut self,
        tree: &mut mir::Tree,
        member: dir::LocalNodeId<dir::Member>,
    ) -> CompilerResult<()> {
        let node = member.into_global_any(self.module);
        let Some(symbol) = self.symbol_declared_at(node)? else {
            return Err(CompilerError::Internal {
                message: "a missing symbol for one associated const".to_string(),
            });
        };

        // skip a memory-kind const, a compile-time term without runtime storage
        let ty = self.symbol_type(symbol)?;
        if self.argument_memory_kind(ty)?.is_some() {
            return Ok(());
        }
        let Some(term) = self.module_constant(symbol)?.filter(Self::is_constant_term) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "an associated const '{}' without its static term",
                    self.symbol_path(symbol)?
                ),
            });
        };

        self.declare_constant_global(tree, symbol, &term)
    }

    /// Declare the global behind one constant binding.
    fn declare_constant(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // declare the constant when its value evaluates
        if let Some(term) = self.module_constant(symbol)?.filter(Self::is_constant_term) {
            return self.declare_constant_global(tree, symbol, &term);
        }

        // declare a mutable global for a binding the module initializer stores
        let declared = self.symbol_type(symbol)?;
        let ty = self.constant_type(tree, declared)?;
        let name = self.symbol_path(symbol)?;
        let name = self.strings.intern(&name);
        let global = tree.insert(mir::Global::new(
            self.module,
            name,
            ty,
            mir::Mutability::Mutable,
            mir::GlobalInitializer::zero(),
        ));
        self.index_language_declaration(tree, global, symbol);
        self.globals.insert(symbol, Ok(global));

        // store the runtime value from the module initializer
        if let Some(value) = value {
            self.initializers
                .push(ModuleInitializer::Binding { global, value });
        }

        Ok(())
    }

    /// Declare the constant global behind one evaluated term under its qualified name.
    fn declare_constant_global(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        term: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        let declared = self.symbol_type(symbol)?;
        let initializer = self.constant_initializer(tree, term, declared)?;
        let ty = self.constant_type(tree, declared)?;
        let name = self.symbol_path(symbol)?;
        let name = self.strings.intern(&name);
        let global = tree.insert(mir::Global::constant(self.module, name, ty, initializer));
        self.index_language_declaration(tree, global, symbol);
        self.globals.insert(symbol, Ok(global));

        Ok(())
    }

    /// Return the global behind one constant, a foreign one imported on first read.
    pub(in crate::lower) fn constant_global(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Global>>> {
        // import a foreign constant on its first read
        if !self.globals.contains_key(&symbol) {
            if symbol.module_id == self.module || !self.is_constant_binding(symbol)? {
                return Ok(None);
            }
            let name = self.symbol_path(symbol)?;
            let name = self.strings.intern(&name);
            let Some(global) = self.import_global(
                tree,
                symbol.module_id,
                mir::Symbol::named(symbol.module_id, name),
            )?
            else {
                return Ok(None);
            };
            let global = tree.insert(global);
            self.index_language_declaration(tree, global, symbol);
            self.globals.insert(symbol, Ok(global));
        }

        // read the global this module declared or imported
        match self.globals.get(&symbol) {
            Some(Ok(global)) => Ok(Some(*global)),
            // cascade the recorded declaration failure
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => Err(CompilerError::Internal {
                message: format!(
                    "a missing global behind the constant '{}'",
                    self.symbol_path(symbol)?
                ),
            }),
        }
    }

    /// Lower the module initializer storing every runtime binding.
    pub(in crate::lower) fn lower_module_initializer(
        &mut self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<Option<mir::FunctionId>> {
        // skip the initializer when every module binding is constant
        if self.initializers.is_empty() {
            return Ok(None);
        }

        // declare the initializer under its module-qualified name
        let initializers = std::mem::take(&mut self.initializers);
        let void = builder.tree_mut().intern_type(mir::Type::Void);
        let path = &self.state(self.module)?.path;
        let name = format!("{path}.@init");
        let header = builder.function_header(&name).result(void);
        let function = builder.declare_function(header);
        builder.tree_mut().get_mut(function).linkage = mir::Linkage::Export;

        // lower every stored binding into its body
        FunctionLowerer::lower_initializer(self, builder, function, initializers)?;

        Ok(Some(function))
    }

    /// Return whether one symbol names a module-level variable or an associated const.
    pub(in crate::lower) fn is_constant_binding(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let state = self.state(symbol.module_id)?;
        let declared = state.bindings.get_symbol(symbol.local_id);
        let scope = state.bindings.get_scope(declared.scope);

        Ok(match declared.kind {
            dir::SymbolKind::Variable => scope.is_root(),
            dir::SymbolKind::AssociatedConst => true,
            _ => false,
        })
    }

    /// Return the evaluated constant behind one module binding, when one exists.
    fn module_constant(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        // read the evaluated static behind the symbol
        let statics = &self.state(symbol.module_id)?.statics;
        let Some(id) = statics.get_symbol_static_id(symbol) else {
            return Ok(None);
        };

        Ok(statics.get_static_maybe(id.local_id).cloned())
    }

    /// Return whether one static term lowers to a constant initializer.
    fn is_constant_term(term: &dir::StaticTerm) -> bool {
        // accept literal, newtype, and tuple terms
        match term {
            dir::StaticTerm::Literal { .. } => true,
            dir::StaticTerm::Newtype { value, .. } => Self::is_constant_term(value),
            dir::StaticTerm::Tuple { elements } => elements.iter().all(Self::is_constant_term),
            _ => false,
        }
    }

    /// Build the initializer of one constant from its evaluated term.
    fn constant_initializer(
        &mut self,
        tree: &mut mir::Tree,
        term: &dir::StaticTerm,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::GlobalInitializer> {
        match term {
            // initialize scalars at their lowered representation, a singleton holding no bytes
            dir::StaticTerm::Literal { value } => {
                let representation = self.constant_type(tree, ty)?;
                let representation = tree.get(representation).clone();
                let is_singleton = match &representation {
                    mir::Type::Void | mir::Type::Null => true,
                    mir::Type::Struct { fields, .. } => fields.is_empty(),
                    _ => false,
                };
                if is_singleton {
                    return Ok(mir::GlobalInitializer::zero());
                }
                let constant = self.scalar_constant(*value, &representation, self.pointer_bytes)?;

                Ok(mir::GlobalInitializer::Scalar(constant))
            }

            // wrap a newtype backing as a single-field aggregate
            dir::StaticTerm::Newtype {
                ty: instance,
                value,
            } => {
                let dir::Type::Application(application) = self.ty(*instance)? else {
                    return Err(CompilerError::Internal {
                        message: "a newtype constant without its instance".to_string(),
                    });
                };
                let Some(dir::Definition::Newtype(newtype)) =
                    self.definition(application.symbol)?.cloned()
                else {
                    return Err(CompilerError::Internal {
                        message: "a newtype constant without its definition".to_string(),
                    });
                };
                let backing = newtype.backing;
                let backing = self.constant_initializer(tree, value, backing)?;

                Ok(mir::GlobalInitializer::Aggregate(vec![backing]))
            }

            // initialize each tuple element at its own type
            dir::StaticTerm::Tuple { elements } => {
                let dir::Type::Tuple(tuple) = self.ty(ty)? else {
                    return Err(CompilerError::Internal {
                        message: "a tuple constant at a non-tuple type".to_string(),
                    });
                };
                let element_types = self.types(ty.module_id)?.type_ids(tuple.elements).to_vec();
                if element_types.len() != elements.len() {
                    return Err(CompilerError::Internal {
                        message: "a tuple constant with a mismatched arity".to_string(),
                    });
                }
                let mut values = Vec::with_capacity(elements.len());
                for (element, element_type) in elements.iter().zip(element_types) {
                    values.push(self.constant_initializer(tree, element, element_type)?);
                }

                Ok(mir::GlobalInitializer::Aggregate(values))
            }

            // reject the terms the module initializer stores
            _ => Err(CompilerError::Internal {
                message: "a non-constant term behind one module constant".to_string(),
            }),
        }
    }

    /// Lower one constant's type outside any generic context.
    fn constant_type(
        &mut self,
        tree: &mut mir::Tree,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        // lower the type outside any lifetime parameters
        let scope = GenericScope::default();

        self.type_lowerer(tree, &scope).lower(ty)
    }

    /// Build one scalar constant at its lowered representation.
    fn scalar_constant(
        &self,
        literal: dir::Literal,
        representation: &mir::Type,
        pointer_bytes: u8,
    ) -> CompilerResult<mir::Constant> {
        Ok(match (literal, representation) {
            (dir::Literal::Boolean(value), _) => mir::Constant::Boolean { value },
            (
                dir::Literal::Integer(value),
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
                dir::Literal::Integer(value),
                mir::Type::Int {
                    width,
                    is_signed: false,
                },
            ) => mir::Constant::UInt {
                value: value as u128,
                width: *width,
            },
            (dir::Literal::Integer(value), mir::Type::Usize) => mir::Constant::UInt {
                value: value as u128,
                width: 8 * pointer_bytes as u16,
            },
            (dir::Literal::Integer(value), mir::Type::Isize) => mir::Constant::Int {
                value: value as i128,
                width: 8 * pointer_bytes as u16,
                is_signed: true,
            },
            (dir::Literal::Integer(value), mir::Type::Float(format)) => mir::Constant::Float {
                bits: tspp_core::float_to_bits(format.format(), value as f64),
                format: *format,
            },
            (dir::Literal::Float(value), mir::Type::Float(format)) => mir::Constant::Float {
                bits: tspp_core::float_to_bits(format.format(), value),
                format: *format,
            },
            // reject a literal outside its lowered representation
            (_, representation) => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: format!("a module constant at a {representation:?} representation"),
                }
                .into());
            }
        })
    }
}
