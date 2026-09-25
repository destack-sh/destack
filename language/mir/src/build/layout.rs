use std::fmt;

use tspp_core::FxIndexSet;

use crate::{
    ElementLayout, Function, Global, Layout, LayoutId, LayoutShape, LayoutTable, LocalNodeId,
    NewtypeLayout, NodeVisitor, PlaceType, Primitive, Reference, Representation, Scalar,
    ScalarField, Static, StructLayout, Substitution, TargetLayout, TraceMap, Tree, TupleLayout,
    Type, TypeId, Validity, Vector, WitnessTable, resolve_witness_types, walk_function, walk_type,
};

use super::aggregate::Aggregate;
use super::variant::Variant;

/// Target layout construction for one MIR module.
#[derive(Debug)]
pub struct LayoutBuilder<'tree> {
    /// The MIR tree whose types are laid out.
    tree: &'tree Tree,
    /// The layouts computed so far.
    layouts: &'tree mut LayoutTable,
    /// The target ABI layout.
    target: TargetLayout,
    /// The witnesses resolving associated types, absent over a closed tree.
    witnesses: Option<&'tree WitnessTable>,
    /// The types whose layouts are in flight.
    computing: FxIndexSet<TypeId>,
}

/// Failure to construct one physical MIR layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// One required concrete type layout is absent.
    Missing {
        /// The type whose layout is required.
        ty: TypeId,
    },
    /// One MIR type has no supported physical representation.
    Unsupported {
        /// Unsupported representation.
        construct: String,
    },
    /// A type or array length still requires substitution.
    Unresolved(TypeId),
    /// One variant case carries a non-scalar discriminant.
    InvalidDiscriminant {
        /// Invalid discriminant representation.
        constant: String,
    },
    /// One variant niche is absent from its register representation.
    InvalidNiche {
        /// Byte offset of the missing niche scalar.
        offset: u32,
    },
}

impl fmt::Display for LayoutError {
    /// Format one physical layout failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { ty } => write!(formatter, "missing concrete layout for {ty:?}"),
            Self::Unresolved(ty) => write!(formatter, "unresolved layout for {ty:?}"),
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
            Self::InvalidNiche { offset } => {
                write!(
                    formatter,
                    "variant niche at byte offset {offset} is absent from its representation"
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// MIR types reachable from runtime roots.
struct ReachableTypeCollector {
    /// The reachable types in discovery order.
    types: FxIndexSet<TypeId>,
}

impl ReachableTypeCollector {
    /// Create an empty reachable-type traversal.
    fn new() -> Self {
        Self {
            types: FxIndexSet::default(),
        }
    }
}

impl NodeVisitor for ReachableTypeCollector {
    /// Collect value layouts and the storage layouts used by address operations.
    fn visit_function(&mut self, tree: &Tree, id: LocalNodeId<Function>, function: &Function) {
        walk_function(self, tree, id, function);
    }

    /// Collect one type and every type its layout reaches.
    fn visit_type(&mut self, tree: &Tree, id: TypeId, _ty: &Type) {
        // stop reference cycles at their first visited type
        if !self.types.insert(id) {
            return;
        }

        // leave generic declarations and their applications to their instances
        match tree.get(id) {
            Type::Declaration { declaration } if !tree.get(*declaration).generics.is_empty() => {
                return;
            }
            Type::Application { .. } => return,
            _ => {}
        }
        let ty = tree.type_definition(id);
        match ty {
            Type::Pointer { .. } => {}
            _ => walk_type(self, tree, ty),
        }
    }
}

impl<'tree> LayoutBuilder<'tree> {
    /// Create layout construction over one MIR tree and table.
    pub fn new(tree: &'tree Tree, layouts: &'tree mut LayoutTable, target: TargetLayout) -> Self {
        Self {
            tree,
            layouts,
            target,
            witnesses: None,
            computing: FxIndexSet::default(),
        }
    }

    /// Resolve associated types through one witness table.
    pub fn witnesses(mut self, witnesses: &'tree WitnessTable) -> Self {
        self.witnesses = Some(witnesses);

        self
    }

    /// Compute every value layout reachable from runtime MIR roots.
    pub fn layout_reachable_types(&mut self) -> Result<(), LayoutError> {
        // traverse global storage representations
        let mut reachable = ReachableTypeCollector::new();
        for (id, global) in self.tree.iter_nodes::<Global>() {
            NodeVisitor::visit_global(&mut reachable, self.tree, id, global);
        }

        // traverse callable signatures and bodies
        for (id, function) in self.tree.iter_nodes::<Function>() {
            if function.generics.is_empty() {
                NodeVisitor::visit_function(&mut reachable, self.tree, id, function);
            }
        }

        // lay out each storage projection without retaining a tree borrow
        let functions = self
            .tree
            .iter_nodes::<Function>()
            .filter(|(_, function)| function.generics.is_empty())
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for function in functions {
            self.layout_places(function)?;
        }

        // lay out every type the walk reached
        for ty in reachable.types {
            self.layout_reachable_type(ty)?;
        }

        Ok(())
    }

    /// Lay out every storage component selected by a function.
    fn layout_places(&mut self, function: LocalNodeId<Function>) -> Result<(), LayoutError> {
        for block_index in 0..self.tree.get(function).blocks().len() {
            let block = self.tree.get(function).blocks()[block_index];
            for index in 0..self.tree.get(block).instructions.len() {
                let instruction = self.tree.get(block).instructions[index];
                let instruction = self.tree.get(instruction).clone();
                if let Some(place) = instruction.place() {
                    let root = place
                        .root_type(function, self.tree)
                        .unwrap_or_else(|| unreachable!("memory place has no root type"));
                    let mut ty = PlaceType::Value(root);
                    self.layout_reachable_type(root)?;

                    // collect each storage layout used to compute the selected address
                    for projection in &place.path.projections {
                        ty = ty.project(projection, self.tree).unwrap_or_else(|| {
                            unreachable!("invalid memory place projection {projection:?}")
                        });
                        let storage = match ty {
                            PlaceType::Value(ty) => Some(ty),
                            referent => referent.element(self.tree),
                        };
                        if let Some(storage) = storage {
                            self.layout_reachable_type(storage)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute a layout when one reachable MIR type has a value representation.
    fn layout_reachable_type(&mut self, ty: TypeId) -> Result<(), LayoutError> {
        match self.tree.get(ty) {
            // skip types without runtime representations
            Type::Error | Type::Never | Type::FunctionSignature { .. } | Type::Parameter { .. } => {
                Ok(())
            }
            // skip generic declarations, their instances laid out through their applications
            Type::Declaration { declaration }
                if !self.tree.get(*declaration).generics.is_empty() =>
            {
                Ok(())
            }

            // compute one layout for each represented type
            Type::Declaration { .. }
            | Type::Void
            | Type::Null
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float(_)
            | Type::TypeId
            | Type::Dynamic { .. }
            | Type::Application { .. }
            | Type::Reference { .. }
            | Type::Pointer { .. }
            | Type::Slice { .. }
            | Type::Uninit { .. }
            | Type::ManuallyDrop { .. }
            | Type::FixedArray { .. }
            | Type::Tuple { .. }
            | Type::Struct { .. }
            | Type::Newtype { .. }
            | Type::Variant { .. }
            | Type::Vector { .. }
            | Type::Function { .. }
            | Type::FunctionPointer { .. }
            | Type::Witness { .. } => {
                self.layout_type(ty)?;

                Ok(())
            }
        }
    }

    /// Return the cached or newly computed layout of one MIR type.
    pub fn layout_type(&mut self, ty: TypeId) -> Result<LayoutId, LayoutError> {
        // reuse the layout already computed for this type
        if let Some(id) = self.layouts.layout_id(ty) {
            return Ok(id);
        }

        // substitute the requested definition before computing its layout
        let definition = Substitution::resolve(ty, self.tree);
        let definition = match self.witnesses {
            Some(witnesses) => resolve_witness_types(self.tree, witnesses, definition),
            None => definition,
        };
        if definition != ty {
            let layout = self.layout_type(definition)?;
            self.layouts.set_layout_id(ty, layout);

            return Ok(layout);
        }

        // transparent storage forms share their represented layout exactly
        let represented = match self.tree.get(ty) {
            Type::Uninit { value } | Type::ManuallyDrop { value } => Some(*value),
            _ => None,
        };
        if let Some(represented) = represented {
            let id = self.layout_type(represented)?;
            self.layouts.set_layout_id(ty, id);

            return Ok(id);
        }

        // reject recursive inline storage
        if !self.computing.insert(ty) {
            return Err(self.unsupported("value-recursive representation"));
        }
        let layout = self.compute_type(ty);
        self.computing.swap_remove(&ty);
        let layout = layout?;
        let id = self.layouts.insert(layout);
        self.layouts.set_layout_id(ty, id);

        Ok(id)
    }

    /// Compute the layout of one MIR type against the target.
    fn compute_type(&mut self, ty: TypeId) -> Result<Layout, LayoutError> {
        match self.tree.get(ty).clone() {
            // scalars occupy their natural width
            Type::Void | Type::Null => Ok(Layout {
                shape: LayoutShape::None,
                representation: Representation::Memory,
                niche: None,
                size: 0,
                alignment: 1,
                trace_map: TraceMap::Empty,
                uninhabited: false,
            }),
            Type::Boolean => Ok(Layout::scalar(
                Scalar::with_validity(Primitive::Integer { width: 8 }, Validity::new(0, 1)),
                1,
                1,
            )),
            Type::Character => Ok(Layout::scalar(
                Scalar::with_validity(Primitive::Integer { width: 32 }, Validity::new(0, 0x10ffff)),
                4,
                4,
            )),
            Type::Int { width, is_signed } => self.integer_layout(width, is_signed),
            Type::Isize | Type::Usize => {
                let bytes = self.pointer_bytes();
                let scalar = Scalar::new(Primitive::Integer {
                    width: self.target.pointer_bits(),
                });

                Ok(Layout::scalar(scalar, bytes, self.pointer_alignment()))
            }
            Type::Parameter { .. } => Err(LayoutError::Unresolved(ty)),
            Type::TypeId => Ok(Layout::scalar(
                Scalar::new(Primitive::Integer { width: 32 }),
                4,
                4,
            )),
            Type::Float(float) => {
                let bytes = (float.width() as u32).div_ceil(8);
                let scalar = Scalar::new(Primitive::Float(float));

                Ok(Layout::scalar(scalar, bytes, self.natural_alignment(bytes)))
            }

            // lay out one world-relative reference with its permitted values
            Type::Reference { kind, lifetime, .. } => {
                let scalar = self.reference_scalar(kind);

                Ok(Layout {
                    shape: LayoutShape::Scalar,
                    representation: Representation::Scalar(scalar),
                    niche: scalar.niche(0),
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::reference(kind, &lifetime),
                    uninhabited: false,
                })
            }

            // process-local pointers occupy one untraced machine word
            Type::Pointer { .. } => {
                let scalar = Scalar::new(Primitive::Pointer {
                    width: self.target.pointer_bits(),
                });

                Ok(Layout {
                    shape: LayoutShape::Scalar,
                    representation: Representation::Scalar(scalar),
                    niche: None,
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::Empty,
                    uninhabited: false,
                })
            }

            // slices store their base reference followed by one element count
            Type::Slice { kind, lifetime, .. } => {
                let reference = self.reference_scalar(kind);
                let length = Scalar::new(Primitive::Integer {
                    width: self.target.pointer_bits(),
                });
                let representation = Representation::ScalarPair([
                    ScalarField::new(reference, 0),
                    ScalarField::new(length, self.pointer_bytes()),
                ]);

                Ok(Layout {
                    shape: LayoutShape::Slice,
                    representation,
                    niche: reference.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::reference(kind, &lifetime),
                    uninhabited: false,
                })
            }

            // fixed arrays repeat one aligned element representation
            Type::FixedArray {
                element, length, ..
            } => {
                if matches!(self.tree.static_value(length), Static::Parameter(_)) {
                    return Err(LayoutError::Unresolved(ty));
                }
                let length = self
                    .tree
                    .static_value(length)
                    .length()
                    .ok_or_else(|| self.unsupported("an open fixed array"))?;
                let count = u32::try_from(length).map_err(|_| self.unsupported("fixed array"))?;
                let element_layout = self.layout_type(element)?;
                let element_layout = self.layouts.layout(element_layout);
                let element_alignment = element_layout.alignment;
                let stride = element_layout.size.next_multiple_of(element_alignment);
                let size = stride
                    .checked_mul(count)
                    .ok_or_else(|| self.unsupported("fixed array"))?;
                let niche = if count == 0 {
                    None
                } else {
                    element_layout.niche
                };
                let representation = match (element_layout.representation, count) {
                    (Representation::Scalar(scalar), 1) => Representation::Scalar(scalar),
                    (Representation::Scalar(scalar), 2) => Representation::ScalarPair([
                        ScalarField::new(scalar, 0),
                        ScalarField::new(scalar, stride),
                    ]),
                    _ => Representation::Memory,
                };
                let trace_map = TraceMap::repeated(count, stride, element_layout.trace_map.clone());

                Ok(Layout {
                    shape: LayoutShape::Array(ElementLayout {
                        element,
                        stride,
                        count,
                    }),
                    representation,
                    niche,
                    size,
                    alignment: element_alignment,
                    trace_map,
                    uninhabited: count != 0 && element_layout.uninhabited,
                })
            }

            // structs pack their named fields largest alignment first
            Type::Struct { fields, .. } => {
                let mut components = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = self.tree.get(field).clone();
                    components.push((field.name, field.ty));
                }
                let aggregate = Aggregate::new(&components, self)?;

                Ok(Layout {
                    shape: LayoutShape::Struct(StructLayout {
                        fields: aggregate.fields,
                    }),
                    representation: aggregate.representation,
                    niche: aggregate.niche,
                    size: aggregate.size,
                    alignment: aggregate.alignment,
                    trace_map: aggregate.trace_map,
                    uninhabited: aggregate.uninhabited,
                })
            }

            // tuples pack their elements the same way
            Type::Tuple { elements, .. } => {
                let components: Vec<_> = elements.iter().map(|element| (None, *element)).collect();
                let aggregate = Aggregate::new(&components, self)?;

                Ok(Layout {
                    shape: LayoutShape::Tuple(TupleLayout {
                        elements: aggregate.fields,
                    }),
                    representation: aggregate.representation,
                    niche: aggregate.niche,
                    size: aggregate.size,
                    alignment: aggregate.alignment,
                    trace_map: aggregate.trace_map,
                    uninhabited: aggregate.uninhabited,
                })
            }

            // newtypes store transparently as their value type
            Type::Newtype { value, .. } => {
                let backing = self.layout_type(value)?;
                let layout = self.layouts.layout(backing);
                let trace_map = layout.trace_map.clone();

                Ok(Layout {
                    shape: LayoutShape::Newtype(NewtypeLayout {
                        backing_type: value,
                        backing_layout: backing,
                    }),
                    representation: layout.representation,
                    niche: layout.niche,
                    size: layout.size,
                    alignment: layout.alignment,
                    trace_map,
                    uninhabited: layout.uninhabited,
                })
            }

            // variants pack their widest payload behind a direct discriminant
            Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                let variant = Variant::new(discriminant, &cases, self)?;

                variant.layout()
            }

            // vectors store fixed scalar lanes inline
            Type::Vector { element, lanes, .. } => {
                if matches!(self.tree.static_value(lanes), Static::Parameter(_)) {
                    return Err(LayoutError::Unresolved(ty));
                }
                let lanes = self
                    .tree
                    .static_value(lanes)
                    .length()
                    .and_then(|lanes| u32::try_from(lanes).ok())
                    .ok_or_else(|| self.unsupported("vector lane count"))?;
                let element_layout = self.layout_type(element)?;
                let element_layout = self.layouts.layout(element_layout);
                let stride = element_layout
                    .size
                    .next_multiple_of(element_layout.alignment);
                let size = stride
                    .checked_mul(lanes)
                    .ok_or_else(|| self.unsupported("vector"))?;
                let Representation::Scalar(element_scalar) = element_layout.representation else {
                    return Err(self.unsupported("vector element"));
                };
                let alignment = self.natural_alignment(size);

                Ok(Layout {
                    shape: LayoutShape::Vector(ElementLayout {
                        element,
                        stride,
                        count: lanes,
                    }),
                    representation: Representation::Vector(Vector::new(element_scalar, lanes)),
                    niche: None,
                    size,
                    alignment,
                    trace_map: TraceMap::Empty,
                    uninhabited: false,
                })
            }

            // dynamic values store one erased payload reference and dispatch table id
            Type::Dynamic { kind, lifetime, .. } => {
                let payload = self.reference_scalar(kind);
                let table = Scalar::new(Primitive::Integer { width: 32 });
                let representation = Representation::ScalarPair([
                    ScalarField::new(payload, 0),
                    ScalarField::new(table, self.pointer_bytes()),
                ]);

                Ok(Layout {
                    shape: LayoutShape::Dynamic,
                    representation,
                    niche: payload.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::reference(kind, &lifetime),
                    uninhabited: false,
                })
            }

            // closures store a function identity and erased environment reference
            Type::Function { kind, lifetime, .. } => {
                let environment_offset = self.pointer_bytes();
                let environment_trace = TraceMap::reference(kind, &lifetime);
                let function = self.function_scalar();
                let environment = Scalar::new(Primitive::Pointer {
                    width: self.target.pointer_bits(),
                });
                let representation = Representation::ScalarPair([
                    ScalarField::new(function, 0),
                    ScalarField::new(environment, environment_offset),
                ]);

                Ok(Layout {
                    shape: LayoutShape::Function,
                    representation,
                    niche: function.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::nested(environment_offset, environment_trace),
                    uninhabited: false,
                })
            }

            // bare function identities occupy one target word
            Type::FunctionPointer { .. } => {
                let scalar = self.function_scalar();

                Ok(Layout::scalar(
                    scalar,
                    self.pointer_bytes(),
                    self.pointer_alignment(),
                ))
            }

            Type::Declaration { declaration } => {
                let definition = self
                    .tree
                    .get(declaration)
                    .definition
                    .ok_or_else(|| self.unsupported("opaque type"))?;
                let layout = self.layout_type(definition)?;

                Ok(self.layouts.layout(layout).clone())
            }
            // transparent storage forms are handled before layout construction
            Type::Application { .. }
            | Type::Uninit { .. }
            | Type::ManuallyDrop { .. }
            | Type::Error
            | Type::Witness { .. }
            | Type::FunctionSignature { .. } => Err(self.unsupported("a type without a layout")),

            // give the uninhabited type the zero-sized layout
            Type::Never => Ok(Layout {
                shape: LayoutShape::None,
                representation: Representation::Memory,
                niche: None,
                size: 0,
                alignment: 1,
                trace_map: TraceMap::Empty,
                uninhabited: true,
            }),
        }
    }

    /// Return one computed layout by id.
    pub(super) fn layout(&self, id: LayoutId) -> &Layout {
        self.layouts.layout(id)
    }

    /// Return the target pointer width in bytes.
    fn pointer_bytes(&self) -> u32 {
        u32::from(self.target.pointer.size_bytes)
    }

    /// Return the target pointer alignment in bytes.
    fn pointer_alignment(&self) -> u32 {
        u32::from(self.target.pointer.alignment_bytes)
    }

    /// Return the natural alignment for one directly represented byte width.
    fn natural_alignment(&self, bytes: u32) -> u32 {
        bytes.max(1).next_power_of_two()
    }

    /// Compute one fixed-width integer layout.
    fn integer_layout(&self, width: u16, is_signed: bool) -> Result<Layout, LayoutError> {
        if width == 0 {
            return Err(self.unsupported("zero-width integer"));
        }

        // retain wider integers in canonical memory
        let physical_width = match width {
            1..=8 => 8,
            9..=16 => 16,
            17..=32 => 32,
            33..=64 => 64,
            65..=128 => 128,
            _ => {
                let size = u32::from(width).div_ceil(8);

                return Ok(Layout {
                    shape: LayoutShape::Scalar,
                    representation: Representation::Memory,
                    niche: None,
                    size,
                    alignment: self.pointer_alignment(),
                    trace_map: TraceMap::Empty,
                    uninhabited: false,
                });
            }
        };
        let primitive = Primitive::Integer {
            width: physical_width,
        };

        // preserve the logical value range inside the legalized scalar
        let scalar = if width == physical_width {
            Scalar::new(primitive)
        } else {
            let magnitude = 1u128 << (width - 1);
            let validity = if is_signed {
                let physical_mask = Scalar::new(primitive).bit_mask();
                Validity::new(physical_mask - magnitude + 1, magnitude - 1)
            } else {
                Validity::new(0, (magnitude << 1) - 1)
            };

            Scalar::with_validity(primitive, validity)
        };
        let size = u32::from(physical_width).div_ceil(8);

        Ok(Layout::scalar(scalar, size, self.natural_alignment(size)))
    }

    /// Return a world-relative scalar with validity determined by its reference kind.
    fn reference_scalar(&self, kind: Reference) -> Scalar {
        let primitive = Primitive::Pointer {
            width: self.target.pointer_bits(),
        };

        if kind == Reference::Raw {
            Scalar::new(primitive)
        } else {
            Scalar::reference(primitive)
        }
    }

    /// Return one callable identity scalar reserving the nullish words as a niche.
    fn function_scalar(&self) -> Scalar {
        Scalar::reference(Primitive::Integer {
            width: self.target.pointer_bits(),
        })
    }

    /// Return one unsupported physical representation diagnostic.
    fn unsupported(&self, construct: &str) -> LayoutError {
        LayoutError::Unsupported {
            construct: format!("a layout for this {construct}"),
        }
    }
}
