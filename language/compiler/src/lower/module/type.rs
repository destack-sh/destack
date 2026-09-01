use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::LowerState;
use crate::{CompilerError, CompilerResult, LowerError};

impl LowerState<'_> {
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
                message: format!("a missing type {:?} of {}", ty.local_id, ty.module_id),
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

        // select the inherent method carrying the member name
        for member in definition.members() {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            if self.symbol_name(method.symbol)? == Some(name) {
                return Ok(method.symbol);
            }
        }

        // read the extensions declared on the type's root
        let definitions = &self.state(symbol.module_id)?.definitions;
        let root = dir::TypeRoot::Declaration(symbol);

        // select the extension method carrying the member name
        for extension in definitions.root_extensions(root) {
            let Some(extension) = definitions.extension_definition(extension) else {
                continue;
            };
            for member in &extension.members {
                let dir::DefinitionMember::Method(method) = member else {
                    continue;
                };
                if self.symbol_name(method.symbol)? == Some(name) {
                    return Ok(method.symbol);
                }
            }
        }

        Err(LowerError::Unsupported {
            anchor: self.module.into(),
            construct: "a constraint member without an implementing method".to_string(),
        }
        .into())
    }

    /// Return the invocation count one receiver term permits.
    pub(in crate::lower) fn callable_multiplicity(
        &self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Multiplicity> {
        // read the receiver mode
        let mode = match self.ty(receiver)? {
            dir::Type::Literal(dir::Literal::String(text)) => dir::ReceiverMode::from_text(text),
            _ => None,
        };
        let Some(mode) = mode else {
            return Err(CompilerError::Internal {
                message: format!("a receiver term {receiver:?} without a mode literal"),
            });
        };

        Ok(match mode {
            dir::ReceiverMode::Owned => mir::Multiplicity::Once,
            dir::ReceiverMode::Borrowed(_) => mir::Multiplicity::Repeatable,
        })
    }

    /// Return the canonical text of one memory singleton.
    pub(in crate::lower) fn memory_text(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StringId>> {
        let text = match self.ty(ty)? {
            dir::Type::Literal(dir::Literal::String(value)) => Some(value),
            _ => None,
        };

        Ok(text)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        // select the signature type and the module owning its pool
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a non-callable function type".to_string(),
                });
            }
        };

        // unwrap the signature the callable resolved to
        let dir::Type::FunctionSignature(signature) = signature else {
            return Err(CompilerError::Internal {
                message: "a missing signature behind one function type".to_string(),
            });
        };

        Ok((signature, owner))
    }

    /// Return the construction space one constructor instance committed at selection.
    pub(in crate::lower) fn constructor_instance_space(
        &self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
    ) -> CompilerResult<Option<dir::Space>> {
        let Some((module, instance)) = instance else {
            return Ok(None);
        };

        // take the elected space from the first place-kinded argument
        let interned = self.state(module)?.generics.get_instance(instance);
        for binding in &interned.key.arguments {
            if let Some(space) = self.place_space(binding.argument)? {
                return Ok(Some(space));
            }
        }

        Ok(None)
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
        // unwrap the slice this payload points at
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
        let dir::Type::Literal(dir::Literal::Integer(length)) = self.ty(count)? else {
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
        // resolve the enum owning this variant
        let Some(dir::Definition::Enum(definition)) = self.definition(owner)? else {
            return Err(CompilerError::Internal {
                message: "a variant owner without an enum definition".to_string(),
            });
        };

        // select the variant's declaration order in its enum
        let index = definition.variant_position(variant);
        let Some(index) = index else {
            return Err(CompilerError::Internal {
                message: "a variant missing from its owner definition".to_string(),
            });
        };

        Ok(index as u32)
    }
}
