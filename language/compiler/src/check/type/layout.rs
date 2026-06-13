use destack_artifact::TargetArch;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Origin, Substitution};

impl CheckState<'_> {
    /// Check that one type at a representation slot has a layout.
    pub(in crate::check) fn check_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<destack_artifact::DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);

        match self.layout_of(origin, ty)? {
            Answer::Ready(Some(_)) => Ok(Answer::Ready(None)),
            Answer::Ready(None) => {
                let ty = self.format_type(ty);
                let (module, anchor) = self.source_anchor(source);

                let error = CheckError::LayoutNotConcrete { anchor, module, ty };

                Ok(Answer::Ready(Some(error.into())))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Compute and memoize the layout of one type.
    /// Returns ready none when the type has no concrete representation.
    pub(in crate::check) fn layout_of(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::LocalLayoutId>>> {
        let mut in_flight = IndexSet::new();
        self.layout_of_guarded(origin, ty, &mut in_flight)
    }

    /// Compute one layout with the active recursion chain tracked.
    ///
    /// Inline recursion has no finite representation and reads as no layout (and errors).
    fn layout_of_guarded(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::LocalLayoutId>>> {
        // close the represented type first
        let ty = match self.evaluate_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let segment = ty.module_id;

        // reuse memoized layouts
        if let Some(working) = self.layouts.get(&segment)
            && let Some(id) = working.layout_id_for_type(ty)
        {
            return Ok(Answer::Ready(Some(id)));
        }

        // inline recursion has no finite representation
        if !in_flight.insert(ty) {
            return Ok(Answer::Ready(None));
        }
        let layout = self.compute_layout(origin, segment, ty, ty, in_flight);
        in_flight.swap_remove(&ty);

        match layout? {
            Answer::Ready(Some(layout)) => {
                let working = self.layout_segment(segment);
                let id = working.insert_layout(layout);
                working.set_type_layout(ty, id);

                Ok(Answer::Ready(Some(id)))
            }
            Answer::Ready(None) => Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Return the layout segment for one module.
    fn layout_segment(&mut self, module: ModuleId) -> &mut dir::LayoutSegment {
        self.layouts
            .entry(module)
            .or_insert_with(|| dir::LayoutSegment::new(module))
    }

    /// Compute the layout of one reduced type.
    ///
    /// The qualified root keeps the outer memory forms so `this` binds
    /// the instantiated form inside the laid out declaration.
    fn compute_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        ty: dir::GlobalTypeId,
        qualified: dir::GlobalTypeId,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let pointer_bytes = self.target_pointer_bytes()?;

        match self.ty(ty)? {
            // single-valued types store nothing standalone
            dir::Type::Never | dir::Type::Void | dir::Type::Undefined | dir::Type::Null => {
                Ok(Answer::Ready(Some(dir::Layout::unit())))
            }
            // erased values carry a dispatch header
            dir::Type::Any | dir::Type::Unknown | dir::Type::Object | dir::Type::Dynamic(_) => {
                Ok(Answer::Ready(Some(dir::Layout::dynamic(pointer_bytes))))
            }
            dir::Type::Primitive(primitive) => Ok(Answer::Ready(primitive.layout(pointer_bytes))),
            dir::Type::Literal(literal) => Ok(Answer::Ready(literal.layout())),
            dir::Type::Range(_) => Ok(Answer::Ready(Some(dir::Layout::scalar(16, 8, None)))),
            dir::Type::Function(_) | dir::Type::Closure(_) => {
                Ok(Answer::Ready(Some(dir::Layout::closure(pointer_bytes))))
            }
            // indirect forms are pointers, direct forms keep their payload
            dir::Type::Form(form) => {
                let (form, value) = (form.form, form.value);

                match form {
                    dir::Form::Managed | dir::Form::Borrowed { .. } => Ok(Answer::Ready(Some(
                        dir::Layout::pointer_slot(value, pointer_bytes, true),
                    ))),
                    dir::Form::Raw => Ok(Answer::Ready(Some(dir::Layout::pointer_slot(
                        value,
                        pointer_bytes,
                        false,
                    )))),
                    // direct forms keep the qualified root for `this`
                    dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                        self.compute_layout(origin, segment, value, qualified, in_flight)
                    }
                }
            }
            dir::Type::FixedArray(array) => {
                let (element, count) = (array.element, array.count);

                self.fixed_array_layout(origin, segment, element, count, in_flight)
            }
            dir::Type::Slice(slice) => {
                let element = slice.element;

                Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Slice(dir::SliceLayout { element }),
                    size: Some(pointer_bytes * 2),
                    alignment: Some(pointer_bytes),
                    niche: Some(dir::Niche {
                        offset: 0,
                        width: pointer_bytes,
                        start: 1,
                        end: dir::Niche::scalar_max(pointer_bytes),
                    }),
                })))
            }
            dir::Type::Tuple(tuple) => {
                let fields = tuple
                    .elements
                    .iter()
                    .map(|element| (None, element.ty))
                    .collect::<SmallVec<[_; 4]>>();

                self.aggregate_layout(origin, segment, &fields, AggregateShape::Tuple, in_flight)
            }
            dir::Type::Shape(shape) => {
                let fields = shape
                    .fields
                    .iter()
                    .map(|field| (Some(field.key), field.ty))
                    .collect::<SmallVec<[_; 4]>>();

                self.aggregate_layout(origin, segment, &fields, AggregateShape::Struct, in_flight)
            }
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.union_layout(origin, segment, &elements, in_flight)
            }
            dir::Type::Reference(instance) => {
                let instance = instance.clone();

                self.reference_layout(origin, segment, ty, qualified, &instance, in_flight)
            }
            // open or symbolic types have no representation yet
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Compute one fixed array layout.
    fn fixed_array_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        element: dir::GlobalTypeId,
        count: dir::GlobalTypeId,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // close the element layout and the static length
        let slot = match self.slot_layout(origin, segment, element, in_flight)? {
            Answer::Ready(Some(slot)) => slot,
            Answer::Ready(None) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let count = match self.evaluate_root(origin, count)? {
            Answer::Ready(count) => count,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let length = match self.ty(count)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => u32::try_from(*value).ok(),
            _ => None,
        };
        let Some(length) = length else {
            return Ok(Answer::Ready(None));
        };

        let stride = align_to(slot.size, slot.alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Array(dir::ArrayLayout {
                element,
                stride: Some(stride),
                count: Some(length),
            }),
            size: Some(stride.saturating_mul(length)),
            alignment: Some(slot.alignment),
            // the first element's niche carries through
            niche: (length > 0).then_some(slot.niche).flatten(),
        })))
    }

    /// Compute one ordered aggregate layout.
    /// TODO: apply representation decorators to field and aggregate
    /// alignment once decorator capture wires them through.
    fn aggregate_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        fields: &[(Option<dir::StaticKey>, dir::GlobalTypeId)],
        shape: AggregateShape,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let mut offset = 0u32;
        let mut alignment = 1u32;
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut niche: Option<dir::Niche> = None;

        for (key, ty) in fields.iter().copied() {
            let slot = match self.slot_layout(origin, segment, ty, in_flight)? {
                Answer::Ready(Some(slot)) => slot,
                Answer::Ready(None) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let field_offset = align_to(offset, slot.alignment);

            // keep the largest niche shifted to its field offset
            if let Some(slot_niche) = slot.niche {
                let shifted = dir::Niche {
                    offset: field_offset + slot_niche.offset,
                    ..slot_niche
                };
                if niche.is_none_or(|kept| shifted.free_values() > kept.free_values()) {
                    niche = Some(shifted);
                }
            }

            offset = field_offset.saturating_add(slot.size);
            alignment = alignment.max(slot.alignment);
            layout_fields.push(dir::LayoutField {
                key,
                ty,
                layout: slot.id,
                offset: Some(field_offset),
                size: Some(slot.size),
                alignment: Some(slot.alignment),
            });
        }

        let shape = match shape {
            AggregateShape::Struct => dir::LayoutShape::Struct(dir::StructLayout {
                fields: layout_fields,
            }),
            AggregateShape::Object => dir::LayoutShape::Object(dir::ObjectLayout {
                fields: layout_fields,
            }),
            AggregateShape::Tuple => dir::LayoutShape::Tuple(dir::TupleLayout {
                elements: layout_fields,
            }),
        };

        Ok(Answer::Ready(Some(dir::Layout {
            shape,
            size: Some(align_to(offset, alignment)),
            alignment: Some(alignment),
            niche,
        })))
    }

    /// Compute one union layout, hunting niches before adding a tag.
    fn union_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        elements: &[dir::GlobalTypeId],
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // close every variant layout first
        let mut cases = Vec::with_capacity(elements.len());
        let mut slots = SmallVec::<[SlotLayout; 4]>::new();
        for element in elements.iter().copied() {
            let slot = match self.slot_layout(origin, segment, element, in_flight)? {
                Answer::Ready(Some(slot)) => slot,
                Answer::Ready(None) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            cases.push(dir::VariantCaseLayout {
                ty: element,
                layout: slot.id,
            });
            slots.push(slot);
        }

        // pack unit variants into one niched payload when it fits
        let unit_count = slots.iter().filter(|slot| slot.size == 0).count() as u128;
        let payload_count = slots.len() as u128 - unit_count;
        if payload_count == 1 {
            let payload = slots
                .iter()
                .find(|slot| slot.size != 0)
                .copied()
                .unwrap_or_else(|| unreachable!("union payload variant must exist"));

            if let Some(niche) = payload.niche
                && niche.free_values() >= unit_count
            {
                return Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Variant(dir::VariantLayout {
                        tag: dir::VariantTagLayout {
                            ty: None,
                            size: Some(0),
                            alignment: Some(1),
                        },
                        payload_offset: Some(0),
                        variants: cases,
                    }),
                    size: Some(payload.size),
                    alignment: Some(payload.alignment),
                    // the niche is spent encoding the unit variants
                    niche: None,
                })));
            }
        }

        // tag with the smallest unsigned integer that fits every case
        let tag_size = smallest_tag_bytes(slots.len());
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        for slot in &slots {
            payload_size = payload_size.max(slot.size);
            payload_alignment = payload_alignment.max(slot.alignment);
        }
        let payload_offset = align_to(tag_size, payload_alignment);
        let alignment = tag_size.max(payload_alignment);
        let size = align_to(payload_offset.saturating_add(payload_size), alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Variant(dir::VariantLayout {
                tag: dir::VariantTagLayout {
                    ty: None,
                    size: Some(tag_size),
                    alignment: Some(tag_size),
                },
                payload_offset: Some(payload_offset),
                variants: cases,
            }),
            size: Some(size),
            alignment: Some(alignment),
            // free tag values above the case count form the result niche
            niche: Some(dir::Niche {
                offset: 0,
                width: tag_size,
                start: 0,
                end: slots.len().saturating_sub(1) as u128,
            }),
        })))
    }

    /// Compute one nominal reference layout through its definition.
    fn reference_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        ty: dir::GlobalTypeId,
        qualified: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        match self.definition(instance.symbol) {
            // newtypes are transparent over their substituted backing
            Some(dir::Definition::Newtype(definition)) => {
                let backing = definition.value;
                let substitution = self.parameter_substitution(instance)?;
                let backing = self.substituted_type(origin, backing, &substitution)?;
                let slot = match self.slot_layout(origin, segment, backing, in_flight)? {
                    Answer::Ready(Some(slot)) => slot,
                    Answer::Ready(None) => return Ok(Answer::Ready(None)),
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

                Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Newtype(dir::NewtypeLayout {
                        backing_type: backing,
                        backing_layout: slot.id,
                    }),
                    size: Some(slot.size),
                    alignment: Some(slot.alignment),
                    niche: slot.niche,
                })))
            }
            // structs and classes lay their available fields out in order
            Some(definition @ (dir::Definition::Struct(_) | dir::Definition::Class(_))) => {
                let members = definition
                    .instance_fields()
                    .map(|field| (field.key, field.ty, field.condition))
                    .collect::<SmallVec<_>>();
                let shape = match definition {
                    dir::Definition::Struct(_) => AggregateShape::Struct,
                    _ => AggregateShape::Object,
                };

                self.definition_field_layout(
                    origin, segment, ty, qualified, instance, members, shape, in_flight,
                )
            }
            // enums store the smallest unsigned integer fitting their variants
            Some(dir::Definition::Enum(definition)) => {
                let variants = definition
                    .members
                    .iter()
                    .filter(|member| matches!(member, dir::DefinitionMember::Variant(_)))
                    .count();
                let size = smallest_tag_bytes(variants);

                Ok(Answer::Ready(Some(dir::Layout::scalar(
                    size,
                    size,
                    Some(dir::Niche {
                        offset: 0,
                        width: size,
                        start: 0,
                        end: variants.saturating_sub(1) as u128,
                    }),
                ))))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Lay one definition's available fields out in declaration order.
    fn definition_field_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        ty: dir::GlobalTypeId,
        qualified: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        members: SmallVec<[(dir::StaticKey, dir::GlobalTypeId, Option<dir::GlobalTypeId>); 4]>,
        shape: AggregateShape,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // the laid out application binds its own `this`
        let substitution = self
            .parameter_substitution(instance)?
            .with_receiver(qualified);
        let mut fields = SmallVec::<[_; 4]>::new();

        for (key, field, condition) in members {
            // drop fields whose substituted guards decide false
            let available =
                self.decide_member_availability(origin, ty.module_id, condition, &substitution)?;
            match available {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }

            let field = self.substituted_type(origin, field, &substitution)?;
            fields.push((Some(key), field));
        }

        self.aggregate_layout(origin, segment, &fields, shape, in_flight)
    }

    /// Compute one nested layout and summarize it for composition.
    ///
    /// Managed representations occupy pointer slots: the unqualified
    /// default form materializes here, at the representation boundary.
    fn slot_layout(
        &mut self,
        origin: Origin,
        segment: ModuleId,
        ty: dir::GlobalTypeId,
        in_flight: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<SlotLayout>>> {
        // close the slot type to check its default form
        let slot = match self.evaluate_root(origin, ty)? {
            Answer::Ready(slot) => slot,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let managed = match self.decide_managed_representation(origin, slot)? {
            Answer::Ready(managed) => managed,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        if managed {
            let pointer_bytes = self.target_pointer_bytes()?;
            let layout = dir::Layout::pointer_slot(slot, pointer_bytes, true);
            let working = self.layout_segment(slot.module_id);
            let id = working.insert_layout(layout);

            return Ok(Answer::Ready(Some(SlotLayout {
                id,
                size: pointer_bytes,
                alignment: pointer_bytes,
                niche: Some(dir::Niche {
                    offset: 0,
                    width: pointer_bytes,
                    start: 1,
                    end: dir::Niche::scalar_max(pointer_bytes),
                }),
            })));
        }

        let id = match self.layout_of_guarded(origin, ty, in_flight)? {
            Answer::Ready(Some(id)) => id,
            Answer::Ready(None) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // summarize from the owning segment
        let ty = match self.evaluate_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let _ = segment;
        let working = self
            .layouts
            .get(&ty.module_id)
            .unwrap_or_else(|| unreachable!("computed layout must own a layout segment"));
        let layout = working.get_layout(id);
        let (Some(size), Some(alignment)) = (layout.size, layout.alignment) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(SlotLayout {
            id,
            size,
            alignment,
            niche: layout.niche,
        })))
    }

    /// Decide whether one reduced type stores through a managed pointer
    /// by default: classes and object-shaped newtype backings.
    fn decide_managed_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let dir::Type::Reference(instance) = self.ty(ty)? else {
            return Ok(Answer::Ready(false));
        };

        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Class(_)) => return Ok(Answer::Ready(true)),
            // object-shaped newtype backings are managed objects
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return Ok(Answer::Ready(false)),
        };

        // close the backing before judging its shape
        let backing = match self.evaluate_root(origin, backing)? {
            Answer::Ready(backing) => backing,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        Ok(Answer::Ready(matches!(
            self.ty(backing)?,
            dir::Type::Shape(_)
        )))
    }

    /// Substitute applied arguments through one declaration-context type.
    fn substituted_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        substitution: &Substitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if substitution.is_empty() {
            return Ok(ty);
        }
        let source = self.origin_source_node(origin)?;

        self.fold_type(origin.module(), source, ty, substitution.rewrite())
    }

    /// Return the target pointer size.
    pub(in crate::check) fn target_pointer_bytes(&self) -> CompilerResult<u32> {
        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?;
        let bytes = match profile.key.target_arch {
            Some(TargetArch::X86)
            | Some(TargetArch::Armv7)
            | Some(TargetArch::Armv6)
            | Some(TargetArch::Riscv32)
            | Some(TargetArch::Wasm32) => 4,
            _ => 8,
        };

        Ok(bytes)
    }
}

/// One summarized slot layout composed into an aggregate.
#[derive(Debug, Clone, Copy)]
struct SlotLayout {
    /// The inserted layout id.
    id: dir::LocalLayoutId,
    /// The size in bytes.
    size: u32,
    /// The alignment in bytes.
    alignment: u32,
    /// The largest niche of free values.
    niche: Option<dir::Niche>,
}

/// Aggregate layout shape selected by source type syntax.
enum AggregateShape {
    /// Struct storage.
    Struct,
    /// Object storage with a dispatch table header.
    Object,
    /// Tuple storage.
    Tuple,
}

/// Return the smallest unsigned tag width fitting one case count.
fn smallest_tag_bytes(cases: usize) -> u32 {
    match cases {
        0..=0x100 => 1,
        0x101..=0x1_0000 => 2,
        _ => 4,
    }
}

/// Align a byte offset to an alignment.
fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment <= 1 {
        return value;
    }

    value.div_ceil(alignment) * alignment
}
