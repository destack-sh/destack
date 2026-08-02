use destack_core::{FxIndexSet, StringId};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{CompilerError, CompilerResult, LowerError};

/// Target layout construction for one module.
pub(in crate::lower) struct LayoutBuilder<'tree> {
    /// The module anchoring layout diagnostics.
    module: ModuleId,
    /// The tree whose types are laid out.
    tree: &'tree mut mir::Tree,
    /// The layouts computed so far.
    layouts: &'tree mut mir::LayoutTable,
    /// The target pointer width in bytes.
    pointer_bytes: u8,
    /// The types whose layouts are in flight, for cycle detection.
    computing: FxIndexSet<mir::LocalNodeId<mir::Type>>,
}

/// Types requiring layouts that are reachable from runtime roots.
struct ReachableTypeCollector {
    /// The visitor options.
    options: mir::NodeVisitorOptions,
    /// The types already traversed.
    visited: FxIndexSet<mir::LocalNodeId<mir::Type>>,
    /// The types requiring layouts in discovery order.
    types: Vec<mir::LocalNodeId<mir::Type>>,
}

impl ReachableTypeCollector {
    /// Create an empty reachable-type traversal.
    fn new() -> Self {
        Self {
            options: mir::NodeVisitorOptions::default(),
            visited: FxIndexSet::default(),
            types: Vec::new(),
        }
    }
}

impl mir::NodeVisitor for ReachableTypeCollector {
    fn options(&self) -> &mir::NodeVisitorOptions {
        &self.options
    }

    fn visit_type(&mut self, tree: &mir::Tree, id: mir::LocalNodeId<mir::Type>, ty: &mir::Type) {
        // stop reference cycles at their first visited type
        if !self.visited.insert(id) {
            return;
        }

        // retain aggregates whose concrete storage needs a layout
        if matches!(
            ty,
            mir::Type::Struct { .. }
                | mir::Type::Tuple { .. }
                | mir::Type::Newtype { .. }
                | mir::Type::Variant { .. }
        ) {
            self.types.push(id);
        }

        mir::walk_type(self, tree, id, ty);
    }
}

impl<'tree> LayoutBuilder<'tree> {
    /// Create layout construction over one tree and table.
    pub(in crate::lower) fn new(
        module: ModuleId,
        tree: &'tree mut mir::Tree,
        layouts: &'tree mut mir::LayoutTable,
        pointer_bytes: u8,
    ) -> Self {
        Self {
            module,
            tree,
            layouts,
            pointer_bytes,
            computing: FxIndexSet::default(),
        }
    }

    /// Compute every aggregate layout reachable from runtime roots.
    pub(in crate::lower) fn layout_reachable_types(&mut self) -> CompilerResult<()> {
        // traverse named representations
        let mut reachable = ReachableTypeCollector::new();
        for (id, declaration) in self.tree.iter_nodes::<mir::TypeDeclaration>() {
            mir::NodeVisitor::visit_type_declaration(&mut reachable, self.tree, id, declaration);
        }

        // traverse global storage representations
        for (id, global) in self.tree.iter_nodes::<mir::Global>() {
            mir::NodeVisitor::visit_global(&mut reachable, self.tree, id, global);
        }

        // traverse callable signatures and bodies
        for (id, function) in self.tree.iter_nodes::<mir::Function>() {
            mir::NodeVisitor::visit_function(&mut reachable, self.tree, id, function);
        }

        for ty in reachable.types {
            // skip types without runtime representations
            if matches!(
                self.tree.get(ty),
                mir::Type::Error | mir::Type::Never | mir::Type::FunctionSignature { .. }
            ) {
                continue;
            }
            self.layout_type(ty)?;
        }

        Ok(())
    }

    /// Return the cached or newly computed layout of one type.
    pub(in crate::lower) fn layout_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LayoutId> {
        // reuse the layout already computed for this type
        if let Some(id) = self.layouts.types.get(&ty) {
            return Ok(*id);
        }

        // lifetime application shares its base representation exactly
        if let mir::Type::WithLifetimes { base, .. } = self.tree.get(ty) {
            let base = *base;
            let id = self.layout_type(base)?;
            self.layouts.types.insert(ty, id);

            return Ok(id);
        }

        // reject value cycles
        if !self.computing.insert(ty) {
            return Err(CompilerError::Internal {
                message: "a value-recursive type without indirection".to_string(),
            });
        }
        let layout = self.compute_type(ty);
        self.computing.swap_remove(&ty);
        let layout = layout?;
        let id = self.layouts.insert(layout);
        self.layouts.types.insert(ty, id);

        Ok(id)
    }

    /// Compute the layout of one type against the target.
    fn compute_type(&mut self, ty: mir::LocalNodeId<mir::Type>) -> CompilerResult<mir::Layout> {
        match self.tree.get(ty).clone() {
            // lay out scalars at their natural width
            mir::Type::Void => Ok(mir::Layout::scalar(0, 1)),
            mir::Type::Boolean => Ok(mir::Layout::scalar(1, 1)),
            mir::Type::Int { width, .. } => {
                let bytes = (width as u32).div_ceil(8);

                Ok(mir::Layout::scalar(bytes, bytes))
            }
            mir::Type::Isize | mir::Type::Usize => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout::scalar(bytes, bytes))
            }
            // lay out references as one pointer
            mir::Type::Reference { .. } => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout::scalar(bytes, bytes))
            }
            mir::Type::Float(float) => {
                let bytes = (float.width() as u32).div_ceil(8);

                Ok(mir::Layout::scalar(bytes, bytes))
            }

            // lay out function values as a code and environment pointer pair
            mir::Type::Function { .. } => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Function,
                    size: 2 * bytes,
                    alignment: bytes,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // lay out bare function pointers as one target pointer
            mir::Type::FunctionPointer { .. } => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout::scalar(bytes, bytes))
            }

            // lay out dynamics as a pointer-aligned payload and type pair
            mir::Type::Dynamic { .. } => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Dynamic,
                    size: 2 * bytes,
                    alignment: bytes,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // lay out slices as a pointer-aligned data and length descriptor
            mir::Type::Slice { .. } => {
                let bytes = self.pointer_bytes as u32;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Slice,
                    size: 2 * bytes,
                    alignment: bytes,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // repeat the element of fixed arrays at its aligned stride
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let id = self.layout_type(element)?;
                let layout = self.layouts.entries[id.index()].clone();
                let stride = layout.size.next_multiple_of(layout.alignment.max(1));
                let count = u32::try_from(length).map_err(|_| CompilerError::Internal {
                    message: "lowered fixed array length exceeds the layout range".to_string(),
                })?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Array(mir::ElementLayout {
                        element,
                        stride,
                        count,
                    }),
                    size: stride * count,
                    alignment: layout.alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // pack struct fields largest alignment first
            mir::Type::Struct { fields, .. } => {
                let mut components = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = self.tree.get(field).clone();
                    components.push((field.name, field.ty));
                }
                let (fields, size, alignment) = self.pack_fields(&components)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Struct(mir::StructLayout { fields }),
                    size,
                    alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // pack tuple elements the same way
            mir::Type::Tuple { elements, .. } => {
                let components: Vec<_> = elements.iter().map(|element| (None, *element)).collect();
                let (elements, size, alignment) = self.pack_fields(&components)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tuple(mir::TupleLayout { elements }),
                    size,
                    alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // store newtypes transparently as their inner type
            mir::Type::Newtype { inner, .. } => {
                let backing = self.layout_type(inner)?;
                let layout = self.layouts.entries[backing.index()].clone();

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

            // pack the widest variant payload behind a direct discriminant
            mir::Type::Variant {
                discriminant,
                storage,
                cases,
                ..
            } => self.compute_variant(discriminant, storage, &cases),

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a layout for the '{other:?}' type"),
            }
            .into()),
        }
    }

    /// Pack fields largest alignment first, declaration order as the tiebreak.
    fn pack_fields(
        &mut self,
        components: &[(Option<StringId>, mir::LocalNodeId<mir::Type>)],
    ) -> CompilerResult<(Vec<mir::LayoutField>, u32, u32)> {
        // compute each field's own layout in declaration order
        let mut computed = Vec::with_capacity(components.len());
        for (index, (name, ty)) in components.iter().enumerate() {
            let layout = self.layout_type(*ty)?;
            let layout = self.layouts.entries[layout.index()].clone();

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
    fn compute_variant(
        &mut self,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> CompilerResult<mir::Layout> {
        // size the shared storage by the widest case payload
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        for case in cases {
            let layout = self.layout_type(case.ty)?;
            let layout = self.layouts.entries[layout.index()].clone();
            payload_size = payload_size.max(layout.size);
            payload_alignment = payload_alignment.max(layout.alignment);
        }

        // elect a niche when spare payload values can carry the void cases
        if let Some(layout) = self.compute_niche(discriminant, storage, cases)? {
            return Ok(layout);
        }

        // place the discriminant first and the payload at its alignment
        let tag = self.layout_type(discriminant)?;
        let tag = self.layouts.entries[tag.index()].clone();
        let payload_offset = tag.size.next_multiple_of(payload_alignment.max(1));
        let alignment = tag.alignment.max(payload_alignment);
        let size = (payload_offset + payload_size).next_multiple_of(alignment);

        // store every case payload at the shared offset
        let case_layouts = cases
            .iter()
            .map(|case| {
                Ok(mir::VariantCaseLayout {
                    discriminant: case_discriminant(case)?,
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
    fn compute_niche(
        &mut self,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> CompilerResult<Option<mir::Layout>> {
        // require exactly one payload case
        let mut untagged = None;
        for (index, case) in cases.iter().enumerate() {
            if matches!(self.tree.get(case.ty), mir::Type::Void) {
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

        // select the contiguous case range excluding the payload case
        let last_case = cases.len() - 1;
        let (niche_case_start, niche_case_end) = if untagged_case == 0 && last_case > 0 {
            (1, last_case)
        } else if untagged_case == last_case && last_case > 0 {
            (0, last_case - 1)
        } else {
            return Ok(None);
        };

        // require a boolean payload sparing every value above one
        let mir::Type::Boolean = self.tree.get(payload) else {
            return Ok(None);
        };
        let niche_start = 2u128;
        let spare = (u8::MAX as u128) - niche_start + 1;
        let riders = niche_case_end - niche_case_start + 1;
        if riders as u128 > spare {
            return Ok(None);
        }

        // keep the logical discriminants on each case
        let case_layouts = cases
            .iter()
            .map(|case| {
                Ok(mir::VariantCaseLayout {
                    discriminant: case_discriminant(case)?,
                    ty: case.ty,
                    payload_offset: 0,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        let payload_layout = self.layout_type(payload)?;
        let payload_layout = self.layouts.entries[payload_layout.index()].clone();

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
                    niche_case_start: niche_case_start as u32,
                    niche_case_end: niche_case_end as u32,
                    niche_start: mir::Discriminant::from_bits(niche_start),
                },
                cases: case_layouts,
            }),
            size: payload_layout.size,
            alignment: payload_layout.alignment,
            trace_map: mir::TraceMap::Empty,
        }))
    }
}

/// Return the logical discriminant bits of one variant case.
fn case_discriminant(case: &mir::VariantCase) -> CompilerResult<mir::Discriminant> {
    match &case.discriminant {
        mir::Constant::Int { value, .. } => Ok(mir::Discriminant::from_bits(*value as u128)),
        mir::Constant::UInt { value, .. } => Ok(mir::Discriminant::from_bits(*value)),
        mir::Constant::Boolean { value } => Ok(mir::Discriminant::from_bits(*value as u128)),
        other => Err(CompilerError::Internal {
            message: format!("a variant case with the non-scalar tag {other:?}"),
        }),
    }
}
