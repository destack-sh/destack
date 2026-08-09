use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// One declaration instance used for apparent member lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ApparentInstance {
    /// The declaration that owns the apparent members.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The applied declaration arguments.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
}

impl ApparentInstance {
    /// Intern this instance into the checked module.
    pub(in crate::check) fn intern(
        &self,
        _module: ModuleId,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = check.intern_type_ids(&self.arguments)?;
        let instance = dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        };

        check.intern_type(dir::Type::Application(instance))
    }

    /// Return the generic substitution represented by this instance.
    pub(in crate::check) fn substitution(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = check.symbol_template(self.symbol)? else {
            if self.arguments.is_empty() {
                return Ok(TypeSubstitution::default());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric apparent owner {:?} has applied type arguments",
                    self.symbol
                ),
            });
        };

        check.template_substitution(template, &self.arguments)
    }
}

impl CheckState<'_> {
    /// Intern the type used for apparent member lookup into the checked module.
    pub(in crate::check) fn intern_apparent_type(
        &mut self,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(instance) = self.apparent_instance(receiver)? else {
            return Ok(receiver);
        };

        instance.intern(module, self)
    }

    /// Return the declaration instance that owns one receiver's apparent members.
    pub(in crate::check) fn apparent_instance(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        let instance = match self.ty(receiver)? {
            dir::Type::Form(form) => return self.apparent_instance(form.value),
            dir::Type::Variant(variant) => {
                let apparent = match self.tagged_variant_backing(receiver.module_id, &variant)? {
                    Some(backing) => backing,
                    None => variant.owner,
                };

                return self.apparent_instance(apparent);
            }
            dir::Type::Application(instance) => ApparentInstance {
                symbol: self.resolve_symbol_alias(instance.symbol)?,
                arguments: self
                    .type_ids(receiver.module_id, instance.arguments)?
                    .iter()
                    .copied()
                    .collect(),
            },
            ref ty @ (dir::Type::Literal(_) | dir::Type::Primitive(_)) => {
                let Some(item) = ty.member_owner_item() else {
                    return Ok(None);
                };

                ApparentInstance {
                    symbol: self.language_symbol(item)?,
                    arguments: SmallVec::new(),
                }
            }
            dir::Type::Array(array) => ApparentInstance {
                symbol: self.language_symbol(dir::LanguageItem::Array)?,
                arguments: SmallVec::from_slice(&[array.element]),
            },
            dir::Type::Slice(slice) => ApparentInstance {
                symbol: self.language_symbol(dir::LanguageItem::Slice)?,
                arguments: SmallVec::from_slice(&[slice.element]),
            },
            dir::Type::FixedArray(array) => ApparentInstance {
                symbol: self.language_symbol(dir::LanguageItem::FixedArray)?,
                arguments: SmallVec::from_slice(&[array.element, array.count]),
            },
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }
}
