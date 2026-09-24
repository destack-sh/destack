use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the type table of one module.
    pub(in crate::lower) fn types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&dir::TypeTable<'static>> {
        Ok(&self.state(module)?.types)
    }

    /// Return one type by id as sema recorded it lowering through.
    pub(in crate::lower) fn ty(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        let written = self.written(ty)?;
        let Some(reduced) = self.types(ty.module_id)?.reduction(ty) else {
            return Ok(written);
        };
        let symbol = match &written {
            dir::Type::Application(application) => Some(application.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        };
        if let Some(symbol) = symbol
            && matches!(self.definition(symbol)?, Some(dir::Definition::Newtype(_)))
        {
            return Ok(written);
        }

        self.ty(reduced)
    }

    /// Return the declaration one nominal application or reference names.
    pub(in crate::lower) fn nominal_symbol(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        Ok(match self.ty(ty)? {
            dir::Type::Application(application) => Some(application.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        })
    }

    /// Return one type by id as written.
    pub(in crate::lower) fn written(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a missing type {:?} of {}", ty.local_id, ty.module_id),
            })
    }

    /// Return the invocation count one receiver term permits.
    pub(in crate::lower) fn callable_multiplicity(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Multiplicity> {
        // read the receiver mode
        let mode = match self.ty(receiver)? {
            dir::Type::Literal(dir::Literal::String(text)) => {
                dir::ReceiverMode::from_text(self.strings.get(text))
            }
            _ => None,
        };
        let Some(mode) = mode else {
            return Err(CompilerError::Internal {
                message: format!("a receiver term {receiver:?} without a mode literal"),
            });
        };

        Ok(match mode {
            dir::ReceiverMode::Owned => mir::Multiplicity::Once,
            dir::ReceiverMode::Borrowed { .. } => mir::Multiplicity::Repeatable,
        })
    }

    /// Return the canonical text of one memory singleton.
    pub(in crate::lower) fn memory_text(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StringId>> {
        let text = match self.ty(ty)? {
            dir::Type::Literal(dir::Literal::String(value)) => Some(value),
            _ => None,
        };

        Ok(text)
    }

    /// Return whether one written argument fills a parameter of the given kind.
    pub(in crate::lower) fn argument_fills_kind(
        &mut self,
        kind: dir::GenericParameterKind,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // admit the argument by the parameter's own kind
        Ok(match kind {
            dir::GenericParameterKind::Memory(memory) => {
                self.argument_memory_kind(argument)? == Some(memory)
            }
            dir::GenericParameterKind::Type => true,
        })
    }

    /// Return the member one projection writes with its qualifying declaration.
    pub(in crate::lower) fn member_projection(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::MemberType, Option<dir::GlobalSymbolId>)>> {
        let dir::Type::Member(member) = self.ty(id)? else {
            return Ok(None);
        };
        let member = *self.types(id.module_id)?.member(member);
        let qualifier = match member.qualifier {
            Some(qualifier) => self.nominal_symbol(qualifier)?,
            None => None,
        };

        Ok(Some((member, qualifier)))
    }

    /// Return the memory kind of the associated const one dependent projects.
    pub(in crate::lower) fn dependent_memory_kind(
        &mut self,
        symbol: dir::GlobalSymbolId,
        dependent: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        let Some((member, qualifier)) = self.member_projection(dependent)? else {
            return Ok(None);
        };
        let key = member.key;

        // read the declaring scope: the qualifier's declaration, else the projecting member's owner
        let owner = match qualifier {
            Some(qualifier) => qualifier,
            None => match self.imported_member(symbol)? {
                Some(member) => member.owner,
                None => symbol,
            },
        };
        let Some(definition) = self.definition(owner)? else {
            return Ok(None);
        };
        let constant = definition
            .members()
            .iter()
            .find_map(|declared| match declared {
                dir::DefinitionMember::AssociatedConst(constant) if constant.key == key => {
                    Some(constant.symbol)
                }
                _ => None,
            });
        let Some(constant) = constant else {
            return Ok(None);
        };
        let ty = self.symbol_type(constant)?;

        self.argument_memory_kind(ty)
    }

    /// Return the memory kind one argument term inhabits, none for a value type.
    pub(in crate::lower) fn argument_memory_kind(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        Ok(match self.ty(id)? {
            // build the region pair at the lifetime kind
            dir::Type::Region(_) => Some(dir::MemoryParameter::Region),
            // reserved lifetime, space, and access names write as string literals
            dir::Type::Literal(dir::Literal::String(value)) => {
                if dir::Lifetime::parse(self.strings.get(value)).is_some() {
                    Some(dir::MemoryParameter::Region)
                } else if dir::Access::from_text(self.strings.get(value)).is_some() {
                    Some(dir::MemoryParameter::Access)
                } else {
                    None
                }
            }
            // parameters inhabit their declared kind or the kind their constraint names
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let binding = self
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);
                match (binding.memory_parameter(), binding.constraint) {
                    (Some(kind), _) => Some(kind),
                    (None, Some(constraint))
                        if matches!(self.ty(constraint)?, dir::Type::Application(_)) =>
                    {
                        self.argument_memory_kind(constraint)?
                    }
                    _ => None,
                }
            }
            // name each memory domain's own kind
            dir::Type::Application(instance) => self
                .language_item(instance.symbol)
                .and_then(dir::MemoryParameter::from_language_item),
            // joins inhabit the kind every element shares
            dir::Type::Union(union) => {
                let elements = self.types(id.module_id)?.type_ids(union.elements).to_vec();
                let mut shared = None;
                for element in elements {
                    let kind = self.argument_memory_kind(element)?;
                    if kind.is_none() || shared.is_some_and(|shared| shared != kind) {
                        return Ok(None);
                    }
                    shared = Some(kind);
                }

                shared.flatten()
            }
            _ => None,
        })
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        // read the callable beneath its memory forms
        let mut ty = ty;
        while let dir::Type::Form(form) = self.ty(ty)? {
            ty = form.value;
        }

        // select the signature type and the module owning its pool
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
            other => {
                let detail = match &other {
                    dir::Type::Reference(reference) => {
                        format!(" to '{}'", self.symbol_path(reference.symbol)?)
                    }
                    _ => String::new(),
                };

                return Err(CompilerError::Internal {
                    message: format!("a non-callable '{}' type{detail}", other.variant_name()),
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

    /// Return the template one callable type's signature declares.
    pub(in crate::lower) fn callable_template(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalGenericTemplateId>> {
        // read the callable beneath its memory forms and its aliases
        let mut ty = ty;
        loop {
            match self.ty(ty)? {
                dir::Type::Form(form) => ty = form.value,
                dir::Type::Application(application) => {
                    // take the region from the callable's backing reference
                    if self.language_item(application.symbol) == Some(dir::LanguageItem::Function) {
                        return Ok(None);
                    }
                    let Some(dir::Definition::TypeAlias(_)) =
                        self.definition(application.symbol)?
                    else {
                        break;
                    };
                    ty = self.symbol_type(application.symbol)?;
                }
                _ => break,
            }
        }

        let (signature, module) = self.signature(ty)?;

        Ok(self.types(module)?.signature(signature).template)
    }

    /// Return whether one callable type's signature parks the current fiber.
    pub(in crate::lower) fn signature_parks(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let (signature, owner) = self.signature(ty)?;
        let signature = self
            .state(owner)?
            .types
            .signature_maybe(signature)
            .ok_or_else(|| CompilerError::Internal {
                message: "a callable without its signature".to_string(),
            })?;

        Ok(signature.parks)
    }

    /// Return the library class representing one compiler-primitive type.
    pub(in crate::lower) fn representation_item(ty: &dir::Type) -> Option<dir::LanguageItem> {
        match ty {
            dir::Type::Primitive(dir::PrimitiveType::String) => Some(dir::LanguageItem::String),
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => Some(dir::LanguageItem::BigInt),
            _ => None,
        }
    }

    /// Return one enum variant's declaration position.
    pub(in crate::lower) fn variant_position(
        &mut self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        // resolve the enum owning this variant
        let Some(dir::Definition::Enum(definition)) = self.definition(owner)?.cloned() else {
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
