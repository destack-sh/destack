use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, TypeSubstitution};

/// One declaration instance used for apparent member lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ApparentInstance {
    /// The declaration that owns the apparent members.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The applied declaration arguments.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
}

impl ApparentInstance {
    /// Intern this instance into one component module.
    pub(in crate::check) fn intern(
        &self,
        module: ModuleId,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = check.intern_type_ids(module, &self.arguments)?;
        let instance = dir::GenericInstance {
            symbol: self.symbol,
            arguments,
        };

        check.intern_type(module, dir::Type::Instance(instance))
    }

    /// Return the generic substitution represented by this instance.
    pub(in crate::check) fn substitution(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = check.symbol_template(self.symbol)? else {
            return Ok(TypeSubstitution::default());
        };

        let parameters = check.generic_template_parameters(template);
        let arguments = self
            .arguments
            .iter()
            .copied()
            .take(parameters.len())
            .collect();

        Ok(TypeSubstitution {
            parameters,
            arguments,
            receiver: None,
        })
    }
}

impl CheckState<'_> {
    /// Intern the type used for apparent member lookup into one component module.
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
            dir::Type::EnumMember(member) => return self.apparent_instance(member.owner),
            dir::Type::Instance(instance) => ApparentInstance {
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
