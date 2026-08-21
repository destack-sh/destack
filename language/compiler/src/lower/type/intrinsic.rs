use destack_dir as dir;
use destack_mir as mir;

use crate::lower::TypeLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one intrinsic-backed newtype to its compiler-known representation.
    pub(in crate::lower) fn lower_intrinsic(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<()> {
        let item = self.lowerer.language_item(symbol)?;
        match item {
            // reference the payload storage for a unique handle
            Some(dir::LanguageItem::Unique) => {
                let [payload] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Unique instantiated without its payload".to_string(),
                    });
                };
                let pointee = self.lower_pointee(*payload)?;
                let representation = mir::Type::Reference {
                    kind: mir::ReferenceKind::Unique,
                    lifetime: mir::Lifetime::empty(),
                    storage: mir::Storage::Heap(mir::Space::Local),
                    access: mir::Access::Exclusive,
                    pointee,
                    nullability: mir::Nullability::None,
                };
                self.tree.define_type(ty, representation);

                Ok(())
            }
            // erase the requested constraint behind the dynamic carrier
            Some(dir::LanguageItem::Dynamic) => {
                let [constraint] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Dynamic instantiated without its constraint".to_string(),
                    });
                };
                let constraint = self.lower_dynamic_constraint(*constraint)?;
                self.tree.define_type(
                    ty,
                    mir::Type::Dynamic {
                        kind: mir::ReferenceKind::Managed,
                        lifetime: mir::Lifetime::empty(),
                        constraint,
                        storage: mir::Storage::Heap(mir::Space::Local),
                        access: mir::Access::Mutable,
                        nullability: mir::Nullability::None,
                    },
                );

                Ok(())
            }
            // store the atomic payload in an atomic cell
            Some(dir::LanguageItem::Atomic) => {
                let [value] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Atomic instantiated without its value".to_string(),
                    });
                };
                let value = self.lower(*value)?;
                self.tree.define_type(ty, mir::Type::Atomic { value });

                Ok(())
            }
            // carry the runtime type identity token directly
            Some(dir::LanguageItem::TypeId) => {
                self.tree.define_type(ty, mir::Type::TypeId);

                Ok(())
            }
            // carry the runtime type descriptor handle, erasing the reflected type
            Some(dir::LanguageItem::Type) => {
                self.tree.define_type(ty, mir::Type::TypeDescriptor);

                Ok(())
            }
            // carry the payload representation, initialization is a checker concept
            Some(dir::LanguageItem::MaybeUninit) => {
                let [payload] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "MaybeUninit instantiated without its payload".to_string(),
                    });
                };
                let payload = self.lower(*payload)?;
                let representation = self.tree.get(payload).clone();
                self.tree.define_type(ty, representation);

                Ok(())
            }
            // carry the value representation, leaving the aliasing exemption to emit
            Some(dir::LanguageItem::UnsafeCell) => {
                let [value] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "UnsafeCell instantiated without its value".to_string(),
                    });
                };
                let value = self.lower(*value)?;
                let representation = self.tree.get(value).clone();
                self.tree.define_type(ty, representation);

                Ok(())
            }
            // carry callable values at their declared signature and multiplicity
            Some(dir::LanguageItem::Function) => {
                // read the parameter tuple the instantiation carries
                let [parameters, result, multiplicity] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Function instantiated without its signature".to_string(),
                    });
                };
                let dir::Type::Tuple(tuple) = self.lowerer.ty(*parameters)? else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a function value without a parameter tuple".to_string(),
                    }
                    .into());
                };

                // lower every tuple element into a signature parameter
                let elements = self
                    .lowerer
                    .tuple_element_types(parameters.module_id, &tuple)?;
                let mut lowered = Vec::with_capacity(elements.len());
                for element in elements {
                    lowered.push(mir::SignatureParameter::new(mir::TypeId::from(
                        self.lower(element)?,
                    )));
                }

                // intern the signature the callable answers at
                let result = mir::TypeId::from(self.lower(*result)?);
                let signature = self.tree.intern_type(mir::Type::FunctionSignature {
                    lifetimes: Vec::new(),
                    parameters: lowered,
                    result,
                });

                // read the multiplicity off its literal name
                let multiplicity = match self.lowerer.ty(*multiplicity)? {
                    dir::Type::Literal(dir::Literal::String(name))
                        if self.lowerer.strings.get(name) == "once" =>
                    {
                        mir::Multiplicity::Once
                    }
                    dir::Type::Literal(dir::Literal::String(name))
                        if self.lowerer.strings.get(name) == "repeatable" =>
                    {
                        mir::Multiplicity::Repeatable
                    }
                    multiplicity => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "function multiplicity {multiplicity:?} names an unknown literal"
                            ),
                        });
                    }
                };

                // define the callable as a managed function reference
                self.tree.define_type(
                    ty,
                    mir::Type::Function {
                        multiplicity,
                        kind: mir::ReferenceKind::Managed,
                        lifetime: mir::Lifetime::empty(),
                        signature: mir::TypeId::from(signature),
                        storage: mir::Storage::Heap(mir::Space::Local),
                        access: mir::Access::Mutable,
                        nullability: mir::Nullability::None,
                    },
                );

                Ok(())
            }
            // define markers as void, like every other zero-sized singleton
            Some(dir::LanguageItem::Phantom) => {
                self.tree.define_type(ty, mir::Type::Void);

                Ok(())
            }
            // reject lifetime markers, which live in MIR lifetime slots alone
            Some(dir::LanguageItem::Lifetime) => Err(CompilerError::Internal {
                message: "a runtime value of the 'memory.Lifetime' marker".to_string(),
            }),
            _ => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: match item {
                    Some(item) => format!("the '{}' intrinsic representation", item.key()),
                    None => "an undeclared intrinsic representation".to_string(),
                },
            }
            .into()),
        }
    }
}
