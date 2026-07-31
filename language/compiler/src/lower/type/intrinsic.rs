use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one intrinsic-backed newtype to its compiler-known representation.
    pub(in crate::lower) fn lower_intrinsic(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<NominalField>> {
        let item = self.lowerer.language_item(symbol)?;
        match item {
            // reference the payload storage for a unique handle
            Some(dir::LanguageItem::Unique) => {
                let [payload] = arguments else {
                    return Err(CompilerError::Internal {
                        message: "Unique instantiated without its payload".to_string(),
                    });
                };
                let payload = self.lowerer.reduced_type(*payload)?;
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

                Ok(Vec::new())
            }
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
