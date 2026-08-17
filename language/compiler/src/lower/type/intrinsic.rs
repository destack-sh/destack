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
                let payload = *payload;
                let representation = match self.lowerer.ty(payload)? {
                    // carry slice payloads in a fat unique descriptor
                    dir::Type::Slice(slice) => {
                        let element = self.lower(slice.element)?;

                        mir::Type::Slice {
                            kind: mir::ReferenceKind::Unique,
                            lifetime: mir::Lifetime::empty(),
                            element,
                            storage: mir::Storage::Heap(mir::Space::Local),
                            access: mir::Access::Exclusive,
                            nullability: mir::Nullability::None,
                        }
                    }
                    _ => {
                        let pointee = self.lower_pointee(payload)?;

                        mir::Type::Reference {
                            kind: mir::ReferenceKind::Unique,
                            lifetime: mir::Lifetime::empty(),
                            storage: mir::Storage::Heap(mir::Space::Local),
                            access: mir::Access::Exclusive,
                            pointee,
                            nullability: mir::Nullability::None,
                        }
                    }
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
