use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, TypeSubstitution, answer};

use super::aggregate::{AggregateLayout, AggregateSlot};
use super::query::LayoutQuery;
use super::scalar::smallest_tag_bytes;

impl LayoutQuery<'_, '_> {
    /// Compute one nominal reference layout through its definition.
    pub(super) fn reference_layout(
        &mut self,
        owner: ModuleId,
        _ty: dir::GlobalTypeId,
        qualified: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        if let Some(item) = self.check.language_item(instance.symbol)?
            && let Some(layout) = self.language_item_layout(owner, item, instance)?
        {
            return Ok(layout);
        }

        match self.check.definition(instance.symbol).cloned() {
            // newtypes are transparent over their substituted backing
            Some(dir::Definition::Newtype(definition)) => {
                let backing = definition.value;
                let substitution = self.check.instance_substitution(owner, instance)?;
                let backing = self.substituted_type(backing, &substitution)?;
                let source = self
                    .check
                    .origin_source_node(self.origin)?
                    .into_global(self.origin.module());
                let Some(layout_id) = answer!(self.slot_layout(owner, backing, source)?) else {
                    return Ok(Answer::Ready(None));
                };
                let (size, alignment, niche) = {
                    let layout = self.layout(owner, layout_id);
                    (layout.size, layout.alignment, layout.niche)
                };

                Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Newtype(dir::NewtypeLayout {
                        backing_type: backing,
                        backing_layout: layout_id,
                    }),
                    size,
                    alignment,
                    niche,
                })))
            }
            // structs and classes lay their fields out in order
            Some(definition @ (dir::Definition::Struct(_) | dir::Definition::Class(_))) => {
                let mut members = SmallVec::new();
                for member in definition.members() {
                    let dir::DefinitionMember::Field(field) = member else {
                        continue;
                    };
                    if field.space != dir::MemberSpace::Instance {
                        continue;
                    }
                    let Some(ty) = answer!(self.check.definition_member_type(member)?) else {
                        continue;
                    };
                    members.push((field.key, ty, field.source));
                }
                let shape = match definition {
                    dir::Definition::Struct(_) => AggregateLayout::Struct,
                    _ => AggregateLayout::Object,
                };

                self.definition_field_layout(owner, qualified, instance, members, shape)
            }
            // enums store the smallest unsigned integer fitting their variants
            Some(dir::Definition::Enum(definition)) => {
                let variants = definition
                    .members
                    .iter()
                    .filter_map(|member| match member {
                        dir::DefinitionMember::Variant(variant) => Some(variant.symbol),
                        _ => None,
                    })
                    .collect::<SmallVec<[_; 8]>>();
                let size = smallest_tag_bytes(variants.len());
                let backing_type = self.enum_backing_type(size)?;
                let source = self
                    .check
                    .origin_source_node(self.origin)?
                    .into_global(self.origin.module());
                let Some(backing_layout) =
                    answer!(self.slot_layout(owner, backing_type, source)?)
                else {
                    return Ok(Answer::Ready(None));
                };
                let variant_count = variants.len();
                let variants = variants
                    .iter()
                    .enumerate()
                    .map(|(index, symbol)| dir::EnumVariantLayout {
                        symbol: *symbol,
                        discriminant: index as u128,
                    })
                    .collect();

                Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Enum(dir::EnumLayout {
                        backing_type,
                        backing_layout,
                        variants,
                    }),
                    size,
                    alignment: size,
                    niche: Some(dir::Niche {
                        offset: 0,
                        width: size,
                        start: variant_count as u128,
                        end: dir::Niche::scalar_max(size),
                    }),
                })))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the inferred unsigned backing type for one enum tag size.
    pub(super) fn enum_backing_type(&mut self, size: u32) -> CompilerResult<dir::GlobalTypeId> {
        let source = self.check.origin_source_node(self.origin)?;
        let width = (size * 8) as u16;
        let ty = dir::Type::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
            width,
            is_signed: false,
        }));

        self.check.intern_type(self.origin.module(), ty)
    }

    /// Lay one definition's stored fields out in declaration order.
    pub(super) fn definition_field_layout(
        &mut self,
        owner: ModuleId,
        qualified: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        members: SmallVec<[(dir::StaticKey, dir::GlobalTypeId, dir::GlobalNodeIdAny); 4]>,
        shape: AggregateLayout,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // the laid out application binds its own `this`
        let substitution = self
            .check
            .instance_substitution(owner, instance)?
            .with_receiver(qualified);
        let mut fields = SmallVec::<[_; 4]>::new();

        for (key, field, source) in members {
            // substitute the applied field type
            let field = self.substituted_type(field, &substitution)?;
            fields.push(AggregateSlot {
                key: Some(key),
                ty: field,
                source: Some(source),
            });
        }

        self.aggregate_layout(owner, &fields, shape)
    }

    /// Compute compiler-defined layout for one intrinsic language item.
    pub(super) fn language_item_layout(
        &mut self,
        owner: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Option<Answer<Option<dir::Layout>>>> {
        let pointer_bytes = self.target_pointer_bytes()?;

        let answer = match item {
            dir::LanguageItem::Vector => self.vector_layout(owner, instance)?,
            dir::LanguageItem::Tensor => {
                let tensor = match self.tensor_layout_input(owner, instance)? {
                    Answer::Ready(Some(tensor)) => tensor,
                    Answer::Ready(None) => return Ok(Some(Answer::Ready(None))),
                    Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
                };

                Answer::Ready(Some(tensor.layout(pointer_bytes)))
            }
            dir::LanguageItem::TensorView => {
                let tensor = match self.tensor_view_layout_input(owner, instance)? {
                    Answer::Ready(Some(tensor)) => tensor,
                    Answer::Ready(None) => return Ok(Some(Answer::Ready(None))),
                    Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
                };

                Answer::Ready(Some(tensor.layout(pointer_bytes)))
            }
            dir::LanguageItem::ComputeDevice
            | dir::LanguageItem::ComputeMesh
            | dir::LanguageItem::ComputeBuffer
            | dir::LanguageItem::ComputeStream
            | dir::LanguageItem::ComputeEvent
            | dir::LanguageItem::ComputeProgram
            | dir::LanguageItem::ComputeKernel
            | dir::LanguageItem::ComputeKernelArgument => Answer::Ready(Some(dir::Layout {
                shape: dir::LayoutShape::Scalar,
                size: pointer_bytes,
                alignment: pointer_bytes,
                niche: Some(dir::Niche::non_null_pointer(pointer_bytes)),
            })),
            _ => return Ok(None),
        };

        Ok(Some(answer))
    }

    /// Substitute applied arguments through one declaration-context type.
    pub(super) fn substituted_type(
        &mut self,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let target = self.origin.module();

        self.check.substitute_type(target, ty, substitution)
    }
}
