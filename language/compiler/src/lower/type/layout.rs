use destack_core::StringId;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Lower the layouts of every aggregate type in the module.
    pub(in crate::lower) fn lower_layouts(
        &self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<()> {
        let pointer_bytes = builder.pointer_bytes();
        let (tree, layouts) = builder.tree_and_layouts_mut();

        // collect the aggregates before mutating the layout table
        let aggregates: Vec<_> = tree
            .iter_nodes::<mir::Type>()
            .filter(|(_, ty)| {
                matches!(
                    ty,
                    mir::Type::Struct { .. }
                        | mir::Type::Tuple { .. }
                        | mir::Type::Newtype { .. }
                        | mir::Type::Variant { .. }
                )
            })
            .map(|(id, _)| id)
            .collect();

        for ty in aggregates {
            self.lower_layout(tree, layouts, pointer_bytes, ty)?;
        }

        Ok(())
    }

    /// Lower the layout of one MIR type into the layout table.
    pub(in crate::lower) fn lower_layout(
        &self,
        tree: &mut mir::Tree,
        layouts: &mut mir::LayoutTable,
        pointer_bytes: u8,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LayoutId> {
        // reuse the layout already computed for this type
        if let Some(id) = layouts.types.get(&ty) {
            return Ok(*id);
        }

        let layout = self.compute_layout(tree, layouts, pointer_bytes, ty)?;
        let id = layouts.insert(layout);
        layouts.types.insert(ty, id);

        Ok(id)
    }

    /// Compute the layout of one MIR type against the target.
    fn compute_layout(
        &self,
        tree: &mut mir::Tree,
        layouts: &mut mir::LayoutTable,
        pointer_bytes: u8,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Layout> {
        match tree.get(ty).clone() {
            // scalars occupy their natural width
            mir::Type::Void => Ok(self.scalar_layout(0, 1)),
            mir::Type::Boolean => Ok(self.scalar_layout(1, 1)),
            mir::Type::Int { width, .. } => {
                let bytes = (width as u32).div_ceil(8);

                Ok(self.scalar_layout(bytes, bytes))
            }
            mir::Type::Isize | mir::Type::Usize => {
                let bytes = pointer_bytes as u32;

                Ok(self.scalar_layout(bytes, bytes))
            }
            // references occupy one pointer, nullish values in the zero page
            mir::Type::Reference { .. } => {
                let bytes = pointer_bytes as u32;

                Ok(self.scalar_layout(bytes, bytes))
            }
            mir::Type::Float(float) => {
                let bytes = (float.width() as u32).div_ceil(8);

                Ok(self.scalar_layout(bytes, bytes))
            }

            // structs pack their named fields largest alignment first
            mir::Type::Struct { fields, .. } => {
                let mut parts = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = tree.get(field).clone();
                    parts.push((field.name, field.ty));
                }
                let (fields, size, alignment) =
                    self.pack_fields(tree, layouts, pointer_bytes, &parts)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Struct(mir::StructLayout { fields }),
                    size,
                    alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // tuples pack their elements the same way
            mir::Type::Tuple { elements, .. } => {
                let parts: Vec<_> = elements.iter().map(|element| (None, *element)).collect();
                let (elements, size, alignment) =
                    self.pack_fields(tree, layouts, pointer_bytes, &parts)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tuple(mir::TupleLayout { elements }),
                    size,
                    alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // newtypes store transparently as their inner type
            mir::Type::Newtype { inner, .. } => {
                let backing = self.lower_layout(tree, layouts, pointer_bytes, inner)?;
                let layout = layouts.entries[backing.index()].clone();

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Newtype(mir::NewtypeLayout {
                        backing_type: inner,
                        backing_layout: backing,
                    }),
                    size: layout.size,
                    alignment: layout.alignment,
                    trace_map: layout.trace_map,
                })
            }

            // variants pack their widest payload behind a direct discriminant
            mir::Type::Variant {
                discriminant,
                storage,
                cases,
                ..
            } => self.variant_layout(tree, layouts, pointer_bytes, discriminant, storage, &cases),

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a layout for the '{other:?}' type"),
            }
            .into()),
        }
    }

    /// Pack fields largest alignment first, declaration order as the tiebreak.
    fn pack_fields(
        &self,
        tree: &mut mir::Tree,
        layouts: &mut mir::LayoutTable,
        pointer_bytes: u8,
        parts: &[(Option<StringId>, mir::LocalNodeId<mir::Type>)],
    ) -> CompilerResult<(Vec<mir::LayoutField>, u32, u32)> {
        // compute each field's own layout in declaration order
        let mut computed = Vec::with_capacity(parts.len());
        for (index, (name, ty)) in parts.iter().enumerate() {
            let layout = self.lower_layout(tree, layouts, pointer_bytes, *ty)?;
            let layout = layouts.entries[layout.index()].clone();

            computed.push((index, *name, *ty, layout));
        }

        // place fields largest alignment first
        computed.sort_by_key(|(index, _, _, layout)| (std::cmp::Reverse(layout.alignment), *index));
        let mut offset = 0u32;
        let mut alignment = 1u32;
        let mut placed = Vec::with_capacity(computed.len());
        for (index, name, ty, layout) in computed {
            offset = offset.next_multiple_of(layout.alignment.max(1));
            alignment = alignment.max(layout.alignment);
            placed.push(mir::LayoutField {
                name,
                ty,
                offset,
                size: layout.size,
                alignment: layout.alignment,
                source_index: Some(index as u32),
            });
            offset += layout.size;
        }

        // restore declaration order for stable field indexing
        placed.sort_by_key(|field| field.source_index);
        let size = offset.next_multiple_of(alignment);

        Ok((placed, size, alignment))
    }

    /// Compute one variant's layout with a direct discriminant encoding.
    fn variant_layout(
        &self,
        tree: &mut mir::Tree,
        layouts: &mut mir::LayoutTable,
        pointer_bytes: u8,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> CompilerResult<mir::Layout> {
        // the widest case payload sizes the shared storage
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        for case in cases {
            let layout = self.lower_layout(tree, layouts, pointer_bytes, case.ty)?;
            let layout = layouts.entries[layout.index()].clone();
            payload_size = payload_size.max(layout.size);
            payload_alignment = payload_alignment.max(layout.alignment);
        }

        // elect a niche when spare payload values can carry the void cases
        if let Some(layout) =
            self.niche_layout(tree, layouts, pointer_bytes, discriminant, storage, cases)?
        {
            return Ok(layout);
        }

        // the discriminant leads, the payload follows at its alignment
        let tag = self.lower_layout(tree, layouts, pointer_bytes, discriminant)?;
        let tag = layouts.entries[tag.index()].clone();
        let payload_offset = tag.size.next_multiple_of(payload_alignment.max(1));
        let alignment = tag.alignment.max(payload_alignment);
        let size = (payload_offset + payload_size).next_multiple_of(alignment);

        // every case stores its payload at the shared offset
        let case_layouts = cases
            .iter()
            .map(|case| {
                Ok(mir::VariantCaseLayout {
                    discriminant: Self::case_discriminant(case)?,
                    ty: case.ty,
                    payload_offset,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        Ok(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant,
                storage,
                encoding: mir::VariantEncoding::Direct {
                    field: mir::DiscriminantField {
                        offset: 0,
                        byte_len: tag.size as u8,
                        bit_offset: 0,
                        bit_len: (tag.size * 8) as u8,
                    },
                },
                cases: case_layouts,
            }),
            size,
            alignment,
            trace_map: mir::TraceMap::Empty,
        })
    }

    /// Elect one niche encoding when a single payload's spare values cover the rest.
    fn niche_layout(
        &self,
        tree: &mut mir::Tree,
        layouts: &mut mir::LayoutTable,
        pointer_bytes: u8,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> CompilerResult<Option<mir::Layout>> {
        // exactly one case may carry a payload; the others ride its spare values
        let mut untagged = None;
        for (index, case) in cases.iter().enumerate() {
            if matches!(tree.get(case.ty), mir::Type::Void) {
                continue;
            }
            if untagged.is_some() {
                return Ok(None);
            }
            untagged = Some((index, case.ty));
        }
        let Some((untagged_case, payload)) = untagged else {
            return Ok(None);
        };

        // booleans spare every value above one
        let mir::Type::Boolean = tree.get(payload) else {
            return Ok(None);
        };
        let niche_start = 2u128;
        let spare = (u8::MAX as u128) - niche_start + 1;
        let riders = cases.len() - 1;
        if riders as u128 > spare {
            return Ok(None);
        }

        // cases keep their logical discriminants; the encoding maps the niche range
        let case_layouts = cases
            .iter()
            .map(|case| {
                Ok(mir::VariantCaseLayout {
                    discriminant: Self::case_discriminant(case)?,
                    ty: case.ty,
                    payload_offset: 0,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        let payload_layout = self.lower_layout(tree, layouts, pointer_bytes, payload)?;
        let payload_layout = layouts.entries[payload_layout.index()].clone();

        Ok(Some(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant,
                storage,
                encoding: mir::VariantEncoding::Niche {
                    field: mir::DiscriminantField {
                        offset: 0,
                        byte_len: payload_layout.size as u8,
                        bit_offset: 0,
                        bit_len: (payload_layout.size * 8) as u8,
                    },
                    untagged_case: untagged_case as u32,
                    niche_case_start: 0,
                    niche_case_end: (cases.len() - 1) as u32,
                    niche_start: mir::Discriminant::from_bits(niche_start),
                },
                cases: case_layouts,
            }),
            size: payload_layout.size,
            alignment: payload_layout.alignment,
            trace_map: mir::TraceMap::Empty,
        }))
    }

    /// Return the logical discriminant bits sealed on one variant case.
    fn case_discriminant(case: &mir::VariantCase) -> CompilerResult<mir::Discriminant> {
        match &case.discriminant {
            mir::Constant::Int { value, .. } => Ok(mir::Discriminant::from_bits(*value as u128)),
            mir::Constant::UInt { value, .. } => Ok(mir::Discriminant::from_bits(*value)),
            mir::Constant::Boolean { value } => Ok(mir::Discriminant::from_bits(*value as u128)),
            other => Err(CompilerError::Internal {
                message: format!("variant case sealed a non-scalar tag {other:?}"),
            }),
        }
    }

    /// Return one scalar layout of the given size and alignment.
    fn scalar_layout(&self, size: u32, alignment: u32) -> mir::Layout {
        mir::Layout {
            shape: mir::LayoutShape::Scalar,
            size,
            alignment,
            trace_map: mir::TraceMap::Empty,
        }
    }
}
