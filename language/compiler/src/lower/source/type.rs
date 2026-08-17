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
                message: format!("missing type {:?} of {}", ty.local_id, ty.module_id),
            })
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
        let actual = self.ty(ty)?;

        // require the canonical singleton the check walk stores
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

    /// Return the library class representing one compiler-primitive type.
    pub(in crate::lower) fn representation_item(ty: &dir::Type) -> Option<dir::LanguageItem> {
        match ty {
            dir::Type::Primitive(dir::PrimitiveType::String) => Some(dir::LanguageItem::String),
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => Some(dir::LanguageItem::BigInt),
            _ => None,
        }
    }

    /// Return the slice one payload dereferences to, when its pointee is unsized.
    pub(in crate::lower) fn slice_pointee(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::SliceType>> {
        let slice = match self.ty(id)? {
            dir::Type::Slice(slice) => slice,
            // look through transparent newtype identity
            dir::Type::Application(instance) => {
                let Some(dir::Definition::Newtype(newtype)) = self.definition(instance.symbol)?
                else {
                    return Ok(None);
                };
                let dir::Type::Slice(slice) = self.ty(newtype.backing)? else {
                    return Ok(None);
                };

                slice
            }
            _ => return Ok(None),
        };

        // leave open elements to their instance substitution
        if matches!(self.ty(slice.element)?, dir::Type::Parameter(_)) {
            return Ok(None);
        }

        Ok(Some(slice))
    }

    /// Return the element count behind one fixed array length singleton.
    pub(in crate::lower) fn fixed_array_length(
        &self,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<u64> {
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

    /// Return one enum variant's declaration position.
    pub(in crate::lower) fn variant_position(
        &self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        // select the variant's declaration order in its enum
        let Some(dir::Definition::Enum(definition)) = self.definition(owner)? else {
            return Err(CompilerError::Internal {
                message: "a variant owner without an enum definition".to_string(),
            });
        };
        let index = definition.variant_position(variant);
        let Some(index) = index else {
            return Err(CompilerError::Internal {
                message: "a variant missing from its owner definition".to_string(),
            });
        };

        Ok(index as u32)
    }
}
