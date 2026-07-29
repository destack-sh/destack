use std::fmt;

use destack_core::{FxIndexSet, StringId};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{CompilerError, LowerError};

/// Target layout construction for one MIR module.
#[derive(Debug)]
pub struct LayoutBuilder<'tree> {
    /// The module anchoring layout diagnostics.
    module: ModuleId,
    /// The tree whose types are laid out.
    tree: &'tree mut mir::Tree,
    /// The layouts computed so far.
    layouts: &'tree mut mir::LayoutTable,
    /// The target ABI layout.
    target: mir::TargetLayout,
    /// The types whose layouts are in flight, for cycle detection.
    computing: FxIndexSet<mir::LocalNodeId<mir::Type>>,
}

/// Failure to construct one physical MIR layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// One MIR type has no supported physical representation.
    Unsupported {
        /// Module containing the unsupported type.
        module: ModuleId,
        /// Unsupported representation.
        construct: String,
    },
    /// One variant case carries a non-scalar discriminant.
    InvalidDiscriminant {
        /// Invalid discriminant representation.
        constant: String,
    },
    /// One value type contains itself without indirection.
    Recursive {
        /// Recursive MIR type.
        ty: mir::LocalNodeId<mir::Type>,
    },
}

impl fmt::Display for LayoutError {
    /// Format one physical layout failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { construct, .. } => {
                write!(
                    formatter,
                    "unsupported physical representation: {construct}"
                )
            }
            Self::InvalidDiscriminant { constant } => {
                write!(
                    formatter,
                    "variant case has a non-scalar discriminant: {constant}"
                )
            }
            Self::Recursive { ty } => {
                write!(
                    formatter,
                    "type {ty:?} is value-recursive without indirection"
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// MIR types reachable from runtime roots.
struct ReachableTypeCollector {
    /// The visitor options.
    options: mir::NodeVisitorOptions,
    /// The types already traversed.
    visited: FxIndexSet<mir::LocalNodeId<mir::Type>>,
    /// The reachable types in discovery order.
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

        self.types.push(id);

        mir::walk_type(self, tree, id, ty);
    }
}

impl<'tree> LayoutBuilder<'tree> {
    /// Create layout construction over one MIR tree and table.
    pub fn new(
        module: ModuleId,
        tree: &'tree mut mir::Tree,
        layouts: &'tree mut mir::LayoutTable,
        target: mir::TargetLayout,
    ) -> Self {
        Self {
            module,
            tree,
            layouts,
            target,
            computing: FxIndexSet::default(),
        }
    }

    /// Compute every value layout reachable from runtime MIR roots.
    pub fn layout_reachable_types(&mut self) -> Result<(), LayoutError> {
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
            self.layout_reachable_type(ty)?;
        }

        Ok(())
    }

    /// Compute a layout when one reachable MIR type has a value representation.
    fn layout_reachable_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<(), LayoutError> {
        match self.tree.get(ty) {
            // skip types without runtime representations
            mir::Type::Error | mir::Type::Never | mir::Type::FunctionSignature { .. } => Ok(()),

            // compute one layout for each represented type
            mir::Type::Void
            | mir::Type::Boolean
            | mir::Type::Character
            | mir::Type::Int { .. }
            | mir::Type::Isize
            | mir::Type::Usize
            | mir::Type::Float(_)
            | mir::Type::TypeDescriptor
            | mir::Type::TypeId
            | mir::Type::Atomic { .. }
            | mir::Type::Dynamic { .. }
            | mir::Type::WithLifetimes { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Slice { .. }
            | mir::Type::Uninit { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::FixedArray { .. }
            | mir::Type::Tuple { .. }
            | mir::Type::Struct { .. }
            | mir::Type::Newtype { .. }
            | mir::Type::Variant { .. }
            | mir::Type::Vector { .. }
            | mir::Type::Tensor { .. }
            | mir::Type::TensorView { .. }
            | mir::Type::Function { .. }
            | mir::Type::FunctionPointer { .. }
            | mir::Type::Continuation { .. }
            | mir::Type::Waiter { .. } => {
                self.layout_type(ty)?;

                Ok(())
            }
        }
    }

    /// Return the cached or newly computed layout of one type.
    pub(crate) fn layout_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<mir::LayoutId, LayoutError> {
        // reuse the layout already computed for this type
        if let Some(id) = self.layouts.types.get(&ty) {
            return Ok(*id);
        }

        // transparent storage forms share their represented layout exactly
        let represented = match self.tree.get(ty) {
            mir::Type::Atomic { value }
            | mir::Type::WithLifetimes { base: value, .. }
            | mir::Type::Uninit { value }
            | mir::Type::ManuallyDrop { value } => Some(*value),
            _ => None,
        };
        if let Some(represented) = represented {
            let id = self.layout_type(represented)?;
            self.layouts.types.insert(ty, id);

            return Ok(id);
        }

        // reject value cycles
        if !self.computing.insert(ty) {
            return Err(LayoutError::Recursive { ty });
        }
        let layout = self.compute_type(ty);
        self.computing.swap_remove(&ty);
        let layout = layout?;
        let id = self.layouts.insert(layout);
        self.layouts.types.insert(ty, id);

        Ok(id)
    }

    /// Compute the layout of one MIR type against the target.
    fn compute_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<mir::Layout, LayoutError> {
        match self.tree.get(ty).clone() {
            // scalars occupy their natural width
            mir::Type::Void => Ok(mir::Layout {
                shape: mir::LayoutShape::None,
                size: 0,
                alignment: 1,
                trace_map: mir::TraceMap::Empty,
            }),
            mir::Type::Boolean => Ok(mir::Layout::scalar(1, 1)),
            mir::Type::Character => Ok(mir::Layout::scalar(4, 4)),
            mir::Type::Int { width, .. } => {
                let bytes = (width as u32).div_ceil(8);
                let alignment = Self::scalar_alignment(bytes);

                Ok(mir::Layout::scalar(bytes, alignment))
            }
            mir::Type::Isize | mir::Type::Usize | mir::Type::TypeDescriptor => {
                let bytes = self.pointer_bytes();

                Ok(mir::Layout::scalar(bytes, self.pointer_alignment()))
            }
            mir::Type::TypeId => Ok(mir::Layout::scalar(4, 4)),
            mir::Type::Continuation { .. } | mir::Type::Waiter { .. } => {
                let bytes = u64::BITS.div_ceil(8);

                Ok(mir::Layout::scalar(bytes, Self::scalar_alignment(bytes)))
            }
            mir::Type::Float(float) => {
                let bytes = (float.width() as u32).div_ceil(8);

                Ok(mir::Layout::scalar(bytes, Self::scalar_alignment(bytes)))
            }

            // references occupy one pointer, nullish values in the zero page
            mir::Type::Reference { kind, space, .. } => Ok(mir::Layout {
                shape: mir::LayoutShape::Scalar,
                size: self.pointer_bytes(),
                alignment: self.pointer_alignment(),
                trace_map: Self::reference_trace(kind, space),
            }),

            // slices store their base reference followed by one element count
            mir::Type::Slice { kind, space, .. } => Ok(mir::Layout {
                shape: mir::LayoutShape::Slice,
                size: self.pointer_bytes() * 2,
                alignment: self.pointer_alignment(),
                trace_map: Self::reference_trace(kind, space),
            }),

            // fixed arrays repeat one aligned element representation
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let count = u32::try_from(length).map_err(|_| self.unsupported("fixed array"))?;
                let element_layout = self.layout_type(element)?;
                let element_layout = self.layouts.layout(element_layout).clone();
                let stride = element_layout
                    .size
                    .next_multiple_of(element_layout.alignment);
                let size = stride
                    .checked_mul(count)
                    .ok_or_else(|| self.unsupported("fixed array"))?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Array(mir::ElementLayout {
                        element,
                        stride,
                        count,
                    }),
                    size,
                    alignment: element_layout.alignment,
                    trace_map: Self::repeated_trace(count, stride, element_layout.trace_map),
                })
            }

            // pack struct fields largest alignment first
            mir::Type::Struct { fields, .. } => {
                let mut components = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = self.tree.get(field).clone();
                    components.push((field.name, field.ty));
                }
                let (fields, size, alignment, trace_map) = self.pack_fields(&components)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Struct(mir::StructLayout { fields }),
                    size,
                    alignment,
                    trace_map,
                })
            }

            // pack tuple elements the same way
            mir::Type::Tuple { elements, .. } => {
                let components: Vec<_> = elements.iter().map(|element| (None, *element)).collect();
                let (elements, size, alignment, trace_map) = self.pack_fields(&components)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tuple(mir::TupleLayout { elements }),
                    size,
                    alignment,
                    trace_map,
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

            // vectors store fixed scalar lanes inline
            mir::Type::Vector { element, lanes, .. } => {
                let element_layout = self.layout_type(element)?;
                let element_layout = self.layouts.layout(element_layout).clone();
                let stride = element_layout
                    .size
                    .next_multiple_of(element_layout.alignment);
                let size = stride
                    .checked_mul(lanes)
                    .ok_or_else(|| self.unsupported("vector"))?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Vector(mir::ElementLayout {
                        element,
                        stride,
                        count: lanes,
                    }),
                    size,
                    alignment: element_layout.alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // owning tensors carry one managed storage handle
            mir::Type::Tensor {
                element,
                space,
                shape,
                format,
                sharding,
                ..
            } => {
                let rank = u32::try_from(shape.len()).map_err(|_| self.unsupported("tensor"))?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tensor(mir::TensorLayout {
                        element,
                        format,
                        sharding,
                        rank,
                    }),
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: Self::managed_trace(space),
                })
            }

            // tensor views store a base, offset, dimensions, and strides
            mir::Type::TensorView {
                kind,
                space,
                element,
                shape,
                format,
                sharding,
                ..
            } => {
                let rank =
                    u32::try_from(shape.len()).map_err(|_| self.unsupported("tensor view"))?;
                let words = rank
                    .checked_mul(2)
                    .and_then(|dimensions| dimensions.checked_add(2))
                    .ok_or_else(|| self.unsupported("tensor view"))?;
                let size = self
                    .pointer_bytes()
                    .checked_mul(words)
                    .ok_or_else(|| self.unsupported("tensor view"))?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::TensorView(mir::TensorViewLayout {
                        element,
                        format,
                        sharding,
                        rank,
                    }),
                    size,
                    alignment: self.pointer_alignment(),
                    trace_map: Self::reference_trace(kind, space),
                })
            }

            // dynamic values store a managed payload and dispatch table id
            mir::Type::Dynamic { space, .. } => Ok(mir::Layout {
                shape: mir::LayoutShape::Dynamic,
                size: self.pointer_bytes() * 2,
                alignment: self.pointer_alignment(),
                trace_map: Self::managed_trace(space),
            }),

            // closures store a code pointer and environment value
            mir::Type::Function { environment, .. } => {
                let environment_layout = self.layout_type(environment)?;
                let environment_layout = self.layouts.layout(environment_layout).clone();
                let environment_offset = self.pointer_bytes();

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Function,
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: Self::nested_trace(environment_offset, environment_layout.trace_map),
                })
            }

            // bare function pointers occupy one target pointer
            mir::Type::FunctionPointer { .. } => Ok(mir::Layout::scalar(
                self.pointer_bytes(),
                self.pointer_alignment(),
            )),

            // transparent storage forms are handled before layout construction
            mir::Type::Atomic { .. }
            | mir::Type::WithLifetimes { .. }
            | mir::Type::Uninit { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::Error
            | mir::Type::Never
            | mir::Type::FunctionSignature { .. } => Err(self.unsupported("type")),
        }
    }

    /// Pack fields largest alignment first, declaration order as the tiebreak.
    fn pack_fields(
        &mut self,
        components: &[(Option<StringId>, mir::LocalNodeId<mir::Type>)],
    ) -> Result<(Vec<mir::LayoutField>, u32, u32, mir::TraceMap), LayoutError> {
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
        let mut traces = Vec::new();
        for (index, name, ty, layout) in computed {
            offset = offset.next_multiple_of(layout.alignment.max(1));
            alignment = alignment.max(layout.alignment);
            let trace = Self::nested_trace(offset, layout.trace_map);
            if trace.has_reference() {
                traces.push(trace);
            }
            placed.push(mir::LayoutField {
                name,
                ty,
                offset,
                size: layout.size,
                alignment: layout.alignment,
                source_index: index as u32,
            });
            offset += layout.size;
        }

        // restore declaration order for stable field indexing
        placed.sort_by_key(|field| field.source_index);
        let size = offset.next_multiple_of(alignment);
        let trace_map = Self::composite_trace(traces);

        Ok((placed, size, alignment, trace_map))
    }

    /// Return the target pointer width in bytes.
    fn pointer_bytes(&self) -> u32 {
        u32::from(self.target.pointer.size_bytes)
    }

    /// Return the target pointer alignment in bytes.
    fn pointer_alignment(&self) -> u32 {
        u32::from(self.target.pointer.alignment_bytes)
    }

    /// Return the natural alignment for one scalar byte width.
    fn scalar_alignment(bytes: u32) -> u32 {
        bytes.max(1).next_power_of_two()
    }

    /// Return the trace map for one reference value.
    fn reference_trace(kind: mir::ReferenceKind, space: mir::Space) -> mir::TraceMap {
        // raw pointers never participate in managed tracing or frame relocation
        if kind == mir::ReferenceKind::Raw {
            return mir::TraceMap::Empty;
        }

        // frame references must be rewritten when continuations move
        if space == mir::Space::Frame {
            return mir::TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([0]),
            };
        }

        // only managed references keep heap allocations live
        if kind == mir::ReferenceKind::Managed {
            return Self::managed_trace(space);
        }

        mir::TraceMap::Empty
    }

    /// Return the trace map for one managed reference.
    fn managed_trace(space: mir::Space) -> mir::TraceMap {
        match space {
            mir::Space::Local => mir::TraceMap::Fixed {
                local_offsets: Box::new([0]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([]),
            },
            mir::Space::Shared => mir::TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([0]),
                frame_offsets: Box::new([]),
            },
            mir::Space::Frame => mir::TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([0]),
            },
            mir::Space::Static => mir::TraceMap::Empty,
        }
    }

    /// Offset one nested trace map when it can reach references.
    fn nested_trace(byte_offset: u32, map: mir::TraceMap) -> mir::TraceMap {
        if map.has_reference() {
            mir::TraceMap::Nested {
                byte_offset,
                map: Box::new(map),
            }
        } else {
            mir::TraceMap::Empty
        }
    }

    /// Combine independent trace maps without retaining empty entries.
    fn composite_trace(mut maps: Vec<mir::TraceMap>) -> mir::TraceMap {
        match maps.len() {
            0 => mir::TraceMap::Empty,
            1 => maps.remove(0),
            _ => mir::TraceMap::Composite {
                maps: maps.into_boxed_slice(),
            },
        }
    }

    /// Repeat one element trace map across fixed inline storage.
    fn repeated_trace(count: u32, stride: u32, element: mir::TraceMap) -> mir::TraceMap {
        if count == 0 || !element.has_reference() {
            mir::TraceMap::Empty
        } else {
            mir::TraceMap::Repeated {
                count,
                stride,
                element: Box::new(element),
            }
        }
    }

    /// Return one unsupported physical representation diagnostic.
    fn unsupported(&self, construct: &str) -> LayoutError {
        LayoutError::Unsupported {
            module: self.module,
            construct: format!("a layout for this {construct}"),
        }
    }

    /// Return the logical discriminant bits sealed on one variant case.
    fn case_discriminant(case: &mir::VariantCase) -> Result<mir::Discriminant, LayoutError> {
        match &case.discriminant {
            mir::Constant::Int { value, .. } => Ok(mir::Discriminant::from_bits(*value as u128)),
            mir::Constant::UInt { value, .. } => Ok(mir::Discriminant::from_bits(*value)),
            mir::Constant::Boolean { value } => Ok(mir::Discriminant::from_bits(*value as u128)),
            other => Err(LayoutError::InvalidDiscriminant {
                constant: format!("{other:?}"),
            }),
        }
    }

    /// Compute one variant's layout with a direct discriminant encoding.
    fn compute_variant(
        &mut self,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> Result<mir::Layout, LayoutError> {
        // the widest case payload sizes the shared storage
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
                    discriminant: Self::case_discriminant(case)?,
                    ty: case.ty,
                    payload_offset,
                })
            })
            .collect::<Result<Vec<_>, LayoutError>>()?;

        let encoding = mir::VariantEncoding::Direct {
            field: mir::DiscriminantField {
                offset: 0,
                byte_len: tag.size as u8,
                bit_offset: 0,
                bit_len: (tag.size * 8) as u8,
            },
        };
        let trace_map = self.variant_trace(encoding, &case_layouts)?;

        Ok(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant,
                storage,
                encoding,
                cases: case_layouts,
            }),
            size,
            alignment,
            trace_map,
        })
    }

    /// Elect one niche encoding when a single payload's spare values cover the rest.
    fn compute_niche(
        &mut self,
        discriminant: mir::LocalNodeId<mir::Type>,
        storage: mir::LocalNodeId<mir::Type>,
        cases: &[mir::VariantCase],
    ) -> Result<Option<mir::Layout>, LayoutError> {
        // exactly one case may carry a payload; the others ride its spare values
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
                    discriminant: Self::case_discriminant(case)?,
                    ty: case.ty,
                    payload_offset: 0,
                })
            })
            .collect::<Result<Vec<_>, LayoutError>>()?;

        let payload_layout = self.layout_type(payload)?;
        let payload_layout = self.layouts.entries[payload_layout.index()].clone();

        let encoding = mir::VariantEncoding::Niche {
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
        };
        let trace_map = self.variant_trace(encoding, &case_layouts)?;

        Ok(Some(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant,
                storage,
                encoding,
                cases: case_layouts,
            }),
            size: payload_layout.size,
            alignment: payload_layout.alignment,
            trace_map,
        }))
    }

    /// Build case-selected traces for one variant representation.
    fn variant_trace(
        &mut self,
        encoding: mir::VariantEncoding,
        cases: &[mir::VariantCaseLayout],
    ) -> Result<mir::TraceMap, LayoutError> {
        let mut traces = Vec::with_capacity(cases.len());

        // retain each case map at its selected payload offset
        for case in cases {
            let layout = self.layout_type(case.ty)?;
            let map = self.layouts.layout(layout).trace_map.clone();
            traces.push(mir::VariantTrace {
                discriminant: case.discriminant,
                payload_offset: case.payload_offset,
                map,
            });
        }

        // omit case selection when no case can reach a reference
        if traces.iter().all(|trace| !trace.map.has_reference()) {
            Ok(mir::TraceMap::Empty)
        } else {
            Ok(mir::TraceMap::Variant {
                encoding,
                cases: traces.into_boxed_slice(),
            })
        }
    }
}

impl From<LayoutError> for CompilerError {
    /// Convert physical layout failure into lower phase control flow.
    fn from(error: LayoutError) -> Self {
        match error {
            LayoutError::Unsupported { module, construct } => LowerError::Unsupported {
                anchor: module.into(),
                construct,
            }
            .into(),
            LayoutError::InvalidDiscriminant { constant } => Self::Internal {
                message: format!("variant case sealed a non-scalar tag {constant}"),
            },
            LayoutError::Recursive { ty } => Self::Internal {
                message: format!("type {ty:?} is value-recursive without indirection"),
            },
        }
    }
}
