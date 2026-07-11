use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Return the type used for apparent member lookup.
    pub(in crate::check) fn apparent_type(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some((module, instance)) = self.apparent_instance(receiver)? else {
            return Ok(receiver);
        };

        self.intern_type(module, dir::Type::Instance(instance))
    }

    /// Return the declaration instance that owns one receiver's apparent members,
    pub(in crate::check) fn apparent_instance(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(ModuleId, dir::GenericInstance)>> {
        let module = receiver.module_id;
        let instance = match self.ty(receiver)? {
            dir::Type::Form(form) => return self.apparent_instance(form.value),
            dir::Type::EnumMember(member) => return self.apparent_instance(member.owner),
            dir::Type::Instance(instance) => dir::GenericInstance {
                symbol: self.resolve_symbol_alias(instance.symbol)?,
                arguments: instance.arguments,
            },
            dir::Type::Literal(literal) => {
                let Some(item) = literal.owner_item() else {
                    return Ok(None);
                };

                dir::GenericInstance {
                    symbol: self.language_symbol(item),
                    arguments: dir::TypeListId::EMPTY,
                }
            }
            dir::Type::Primitive(primitive) => {
                let Some(item) = primitive.owner_item() else {
                    return Ok(None);
                };

                dir::GenericInstance {
                    symbol: self.language_symbol(item),
                    arguments: dir::TypeListId::EMPTY,
                }
            }
            dir::Type::Array(array) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::Array),
                arguments: self.intern_type_ids(module, &[array.element])?,
            },
            dir::Type::Slice(slice) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::Slice),
                arguments: self.intern_type_ids(module, &[slice.element])?,
            },
            dir::Type::FixedArray(array) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::FixedArray),
                arguments: self.intern_type_ids(module, &[array.element, array.count])?,
            },
            _ => return Ok(None),
        };

        Ok(Some((module, instance)))
    }
}
