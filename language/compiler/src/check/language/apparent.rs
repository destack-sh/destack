use destack_dir as dir;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Return the declaration instance that owns one receiver's apparent members.
    pub(in crate::check) fn apparent_instance(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GenericInstance>> {
        let instance = match self.ty(receiver)? {
            dir::Type::Form(form) => return self.apparent_instance(form.value),
            dir::Type::EnumMember(member) => return self.apparent_instance(member.owner),
            dir::Type::Instance(instance) => {
                let symbol = instance.symbol;
                let arguments = instance.arguments.clone();

                dir::GenericInstance {
                    symbol: self.resolve_symbol_alias(symbol)?,
                    arguments,
                }
            }
            dir::Type::Literal(literal) => {
                let Some(item) = literal.owner_item() else {
                    return Ok(None);
                };

                dir::GenericInstance {
                    symbol: self.language_symbol(item),
                    arguments: Vec::new(),
                }
            }
            dir::Type::Primitive(primitive) => {
                let Some(item) = primitive.owner_item() else {
                    return Ok(None);
                };

                dir::GenericInstance {
                    symbol: self.language_symbol(item),
                    arguments: Vec::new(),
                }
            }
            dir::Type::Array(array) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::Array),
                arguments: vec![array.element],
            },
            dir::Type::Slice(slice) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::Slice),
                arguments: vec![slice.element],
            },
            dir::Type::FixedArray(array) => dir::GenericInstance {
                symbol: self.language_symbol(dir::LanguageItem::FixedArray),
                arguments: vec![array.element, array.count],
            },
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }
}
