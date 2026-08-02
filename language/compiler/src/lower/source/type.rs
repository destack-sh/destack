use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return the type table of one module.
    pub(in crate::lower) fn types(
        &self,
        module: ModuleId,
    ) -> CompilerResult<&dir::TypeTable<'static>> {
        Ok(&self.state(module)?.types)
    }

    /// Return one type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing type {:?}", ty.local_id),
            })
    }

    /// Return the reduction of one type id.
    pub(in crate::lower) fn reduced_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        Ok(self.types(ty.module_id)?.get_reduced_type_id(ty))
    }

    /// Return the inherent method implementing one constraint member.
    pub(in crate::lower) fn implementing_method(
        &self,
        symbol: dir::GlobalSymbolId,
        name: destack_core::StringId,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: "an erased value without a definition".to_string(),
            });
        };
        for member in definition.members() {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            if self.symbol_name(method.symbol)? == Some(name) {
                return Ok(method.symbol);
            }
        }

        Err(LowerError::Unsupported {
            anchor: self.module.into(),
            construct: "an extension-implemented constraint member".to_string(),
        }
        .into())
    }

    /// Return one canonical singleton in a well-known memory domain.
    pub(in crate::lower) fn memory_literal(
        &self,
        ty: dir::GlobalTypeId,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::MemoryLiteral> {
        let ty = self.reduced_type(ty)?;
        let actual = self.ty(ty)?;
        let dir::Type::Memory(literal) = actual else {
            return Err(CompilerError::Internal {
                message: format!("a {kind:?} singleton left as {actual:?}"),
            });
        };
        if literal.kind_language_item() != kind.language_item() {
            return Err(CompilerError::Internal {
                message: format!("the wrong {kind:?} singleton"),
            });
        }

        Ok(literal)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
            other => {
                return Err(CompilerError::Internal {
                    message: format!("a non-callable function: {other:?}"),
                });
            }
        };
        let dir::Type::FunctionSignature(signature) = signature else {
            return Err(CompilerError::Internal {
                message: "missing a signature behind one function type".to_string(),
            });
        };

        Ok((signature, owner))
    }

    /// Return the library class item representing one compiler-primitive type.
    pub(in crate::lower) fn representation_item(
        ty: &dir::Type,
    ) -> Option<(dir::LanguageItem, Vec<dir::GlobalTypeId>)> {
        match ty {
            dir::Type::Primitive(dir::PrimitiveType::String) => {
                Some((dir::LanguageItem::String, Vec::new()))
            }
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => {
                Some((dir::LanguageItem::BigInt, Vec::new()))
            }
            dir::Type::Array(array) => Some((dir::LanguageItem::Array, vec![array.element])),
            _ => None,
        }
    }

    /// Return the slice one payload dereferences to, when its pointee is unsized.
    ///
    /// Transparent newtype identity over a slice erases into the descriptor.
    /// Open elements resolve with their instances and stay opaque here.
    pub(in crate::lower) fn slice_pointee(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::SliceType>> {
        let reduced = self.reduced_type(id)?;
        let slice = match self.ty(reduced)? {
            dir::Type::Slice(slice) => slice,
            // look through transparent newtype identity
            dir::Type::Application(instance) => {
                let Some(dir::Definition::Newtype(newtype)) = self.definition(instance.symbol)?
                else {
                    return Ok(None);
                };
                let backing = self.reduced_type(newtype.backing)?;
                let dir::Type::Slice(slice) = self.ty(backing)? else {
                    return Ok(None);
                };

                slice
            }
            _ => return Ok(None),
        };

        // leave open elements to their instance substitution
        let element = self.reduced_type(slice.element)?;
        if matches!(self.ty(element)?, dir::Type::Parameter(_)) {
            return Ok(None);
        }

        Ok(Some(slice))
    }

    /// Return the element count behind one fixed array length singleton.
    pub(in crate::lower) fn fixed_array_length(
        &self,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<u64> {
        let count = self.reduced_type(count)?;
        let dir::Type::Literal(dir::ScalarLiteral::Integer(length)) = self.ty(count)? else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a fixed array with an open length".to_string(),
            }
            .into());
        };

        u64::try_from(length).map_err(|_| CompilerError::Internal {
            message: "a fixed array closed at a negative length".to_string(),
        })
    }

    /// Return one enum or Tagged variant's declaration position.
    pub(in crate::lower) fn variant_position(
        &self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        // select the declaration order owned by the variant family
        let index = match self.definition(owner)? {
            Some(dir::Definition::Enum(definition)) => definition.variant_position(variant),
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => {
                definition.tagged_variant_position(variant)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "a variant owner without a variant definition".to_string(),
                });
            }
        };
        let Some(index) = index else {
            return Err(CompilerError::Internal {
                message: "a variant missing from its owner definition".to_string(),
            });
        };

        Ok(index as u32)
    }
}
