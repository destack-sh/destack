use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::TypeLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one intrinsic newtype application to its known representation.
    pub(in crate::lower) fn lower_intrinsic(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<mir::TypeId> {
        // dispatch on the language item the newtype names
        let item = self.lower.language_item(symbol);
        match item {
            // rewrite the access of the reference the accessor reads
            Some(dir::LanguageItem::WithAccess) => {
                let [reference, access] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "WithAccess instantiated without its arguments".to_string(),
                    });
                };
                let reference = self.lower(*reference)?;
                let access = self.lower_access(*access)?;
                let mut rewritten = self.tree.get(reference).clone();
                if rewritten.reference_access().is_none() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "an access form over an open reference".to_string(),
                    }
                    .into());
                }
                match &mut rewritten {
                    mir::Type::Dynamic { access: slot, .. }
                    | mir::Type::Reference { access: slot, .. }
                    | mir::Type::Slice { access: slot, .. }
                    | mir::Type::Function { access: slot, .. }
                    | mir::Type::Pointer { access: slot, .. } => *slot = access,
                    _ => unreachable!("an access form over a type without an access slot"),
                }

                Ok(self.tree.intern_type(rewritten))
            }
            // reference the payload storage for a unique handle
            Some(dir::LanguageItem::Unique) => {
                let [payload] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Unique instantiated without its payload".to_string(),
                    });
                };
                let pointee = self.lower_pointee(*payload)?;

                Ok(self.tree.intern_type(mir::Type::Reference {
                    kind: mir::Reference::Unique,
                    lifetime: mir::Lifetime::empty(),
                    access: mir::Access::Mutable,
                    pointee,
                }))
            }
            // erase the requested constraint into the dynamic representation
            Some(dir::LanguageItem::Dynamic) => {
                let [constraint] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Dynamic instantiated without its constraint".to_string(),
                    });
                };
                let constraint = match self.lower.ty(*constraint)? {
                    dir::Type::Parameter(_) => self.lower(*constraint)?,
                    _ => self.lower_dynamic_constraint(*constraint)?,
                };

                Ok(self.tree.intern_type(mir::Type::Dynamic {
                    kind: mir::Reference::Managed(mir::Space::Local),
                    lifetime: mir::Lifetime::empty(),
                    constraint,
                    access: mir::Access::Mutable,
                }))
            }
            // keep the runtime type identity, erasing the reflected type
            Some(dir::LanguageItem::Type) => Ok(self.tree.intern_type(mir::Type::TypeId)),
            // preserve the declared initialization or drop state of stored values
            Some(dir::LanguageItem::MaybeUninit | dir::LanguageItem::ManuallyDrop) => {
                let [value] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "a memory form requires one value argument".to_string(),
                    });
                };
                let value = self.lower(*value)?;

                self.insert_storage_form(symbol, value)
            }
            // keep the value representation under an aliasing exemption
            Some(dir::LanguageItem::UnsafeCell) => {
                let [value] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "UnsafeCell instantiated without its value".to_string(),
                    });
                };
                self.lower(*value)
            }
            // keep callable values at their declared signature and receiver mode
            Some(dir::LanguageItem::Function) => {
                // read the parameter tuple of the instantiation
                let [parameters, result, receiver] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Function instantiated without its signature".to_string(),
                    });
                };
                let dir::Type::Tuple(tuple) = self.lower.ty(*parameters)? else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a function value without a parameter tuple".to_string(),
                    }
                    .into());
                };

                // lower every tuple element into a signature parameter
                let elements = self
                    .lower
                    .tuple_element_types(parameters.module_id, &tuple)?;
                let mut lowered = Vec::with_capacity(elements.len());
                for element in elements {
                    lowered.push(mir::SignatureParameter::new(self.lower(element)?));
                }

                // intern the signature the callable answers at
                let result = self.lower(*result)?;
                let signature = self.tree.intern_type(mir::Type::FunctionSignature {
                    lifetimes: Vec::new(),
                    parameters: lowered,
                    result,
                });

                // read the invocation count off the receiver mode
                let multiplicity = self.lower.callable_multiplicity(*receiver)?;

                // define the callable as a managed function reference
                Ok(self.tree.intern_type(mir::Type::Function {
                    multiplicity,
                    kind: mir::Reference::Managed(mir::Space::Local),
                    lifetime: mir::Lifetime::empty(),
                    signature,
                    access: mir::Access::Mutable,
                }))
            }
            // wrap a pinned pointer over its own representation, its payload deciding copy
            Some(dir::LanguageItem::Pin) => {
                let [pointer] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Pin instantiated without its pointer".to_string(),
                    });
                };
                let value = self.lower(*pointer)?;

                Ok(self.tree.intern_type(mir::Type::Newtype { value }))
            }
            // lay lanes of one element out at a closed lane count
            Some(dir::LanguageItem::Vector) => {
                let [element, lanes] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Vector instantiated without its element and lanes".to_string(),
                    });
                };
                let element = self.lower(*element)?;
                let mir::GenericArgument::Value(lanes) = self.lower_generic_argument(*lanes)?
                else {
                    return Err(CompilerError::Internal {
                        message: "a vector lane count outside the value domain".to_string(),
                    });
                };

                Ok(self.tree.intern_type(mir::Type::Vector { element, lanes }))
            }
            // address one value through a raw pointer
            Some(dir::LanguageItem::Pointer) => {
                let [pointee] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "a pointer requires one pointee argument".to_string(),
                    });
                };
                let pointee = self.lower(*pointee)?;

                Ok(self.tree.intern_type(mir::Type::Pointer {
                    pointee,
                    access: mir::Access::Mutable,
                }))
            }
            // define markers as void, like every other zero-sized singleton
            Some(dir::LanguageItem::Phantom) => Ok(self.tree.intern_type(mir::Type::Void)),
            // define profile instruments as zero-sized singletons, naming them in their types
            Some(dir::LanguageItem::ProfileCounter | dir::LanguageItem::ProfileSampler) => {
                Ok(self.tree.intern_type(mir::Type::Void))
            }
            // reject region markers, which live in MIR lifetime slots alone
            Some(dir::LanguageItem::Region) => Err(CompilerError::Internal {
                message: "a runtime value of the 'memory.Region' marker".to_string(),
            }),
            // reject every remaining intrinsic
            _ => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: match item {
                    Some(item) => format!("the '{}' intrinsic representation", item.key()),
                    None => "an undeclared intrinsic representation".to_string(),
                },
            }
            .into()),
        }
    }
}
