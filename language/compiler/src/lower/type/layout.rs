use std::fmt;

use destack_core::FxIndexSet;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{CompilerError, LowerError};

use super::aggregate::Aggregate;
use super::variant::Variant;

/// Target layout construction for one MIR module.
#[derive(Debug)]
pub struct LayoutBuilder<'tree> {
    /// The module anchoring layout diagnostics.
    module: ModuleId,
    /// The MIR tree whose types are laid out.
    tree: &'tree mir::Tree,
    /// The layouts computed so far.
    layouts: &'tree mut mir::LayoutTable,
    /// The target ABI layout.
    target: mir::TargetLayout,
    /// The types whose layouts are in flight.
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
    /// One variant niche is absent from its register representation.
    InvalidNiche {
        /// Byte offset of the missing niche scalar.
        offset: u32,
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
            Self::InvalidNiche { offset } => {
                write!(
                    formatter,
                    "variant niche at byte offset {offset} is absent from its representation"
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
    /// The MIR visitor options.
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
        tree: &'tree mir::Tree,
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
            | mir::Type::Application { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Pointer { .. }
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
            | mir::Type::FunctionPointer { .. } => {
                self.layout_type(ty)?;

                Ok(())
            }
        }
    }

    /// Return the cached or newly computed layout of one MIR type.
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
            | mir::Type::Application { base: value, .. }
            | mir::Type::Uninit { value }
            | mir::Type::ManuallyDrop { value } => Some(*value),
            _ => None,
        };
        if let Some(represented) = represented {
            let id = self.layout_type(represented)?;
            self.layouts.types.insert(ty, id);

            return Ok(id);
        }

        // reject recursive inline storage
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
                representation: mir::Representation::Memory,
                niche: None,
                size: 0,
                alignment: 1,
                trace_map: mir::TraceMap::Empty,
            }),
            mir::Type::Boolean => Ok(mir::Layout::scalar(
                mir::Scalar::with_validity(
                    mir::Primitive::Integer { width: 8 },
                    mir::Validity::new(0, 1),
                ),
                1,
                1,
            )),
            mir::Type::Character => Ok(mir::Layout::scalar(
                mir::Scalar::with_validity(
                    mir::Primitive::Integer { width: 32 },
                    mir::Validity::new(0, 0x10ffff),
                ),
                4,
                4,
            )),
            mir::Type::Int { width, is_signed } => self.integer_layout(width, is_signed),
            mir::Type::Isize | mir::Type::Usize => {
                let bytes = self.pointer_bytes();
                let scalar = mir::Scalar::new(mir::Primitive::Integer {
                    width: self.target.pointer_bits(),
                });

                Ok(mir::Layout::scalar(scalar, bytes, self.pointer_alignment()))
            }
            mir::Type::TypeDescriptor => {
                let bytes = self.pointer_bytes();
                let scalar = self.pointer_scalar(mir::Nullability::None);

                Ok(mir::Layout::scalar(scalar, bytes, self.pointer_alignment()))
            }
            mir::Type::TypeId => Ok(mir::Layout::scalar(
                mir::Scalar::new(mir::Primitive::Integer { width: 32 }),
                4,
                4,
            )),
            mir::Type::Float(float) => {
                let bytes = (float.width() as u32).div_ceil(8);
                let scalar = mir::Scalar::new(mir::Primitive::Float(float));

                Ok(mir::Layout::scalar(
                    scalar,
                    bytes,
                    self.natural_alignment(bytes),
                ))
            }

            // references occupy one pointer, nullish values in the zero page
            mir::Type::Reference {
                kind,
                storage,
                nullability,
                ..
            } => {
                let scalar = self.pointer_scalar(nullability);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Scalar,
                    representation: mir::Representation::Scalar(scalar),
                    niche: scalar.niche(0),
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::reference(kind, storage),
                })
            }

            // process-local pointers occupy one untraced machine word
            mir::Type::Pointer { nullability, .. } => {
                let scalar = self.pointer_scalar(nullability);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Scalar,
                    representation: mir::Representation::Scalar(scalar),
                    niche: scalar.niche(0),
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // slices store their base reference followed by one element count
            mir::Type::Slice {
                kind,
                storage,
                nullability,
                ..
            } => {
                let reference = self.pointer_scalar(nullability);
                let length = mir::Scalar::new(mir::Primitive::Integer {
                    width: self.target.pointer_bits(),
                });
                let representation = mir::Representation::ScalarPair([
                    mir::ScalarField::new(reference, 0),
                    mir::ScalarField::new(length, self.pointer_bytes()),
                ]);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Slice,
                    representation,
                    niche: reference.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::reference(kind, storage),
                })
            }

            // fixed arrays repeat one aligned element representation
            mir::Type::FixedArray {
                element, length, ..
            } => {
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
                    (mir::Representation::Scalar(scalar), 1) => mir::Representation::Scalar(scalar),
                    (mir::Representation::Scalar(scalar), 2) => mir::Representation::ScalarPair([
                        mir::ScalarField::new(scalar, 0),
                        mir::ScalarField::new(scalar, stride),
                    ]),
                    _ => mir::Representation::Memory,
                };
                let trace_map =
                    mir::TraceMap::repeated(count, stride, element_layout.trace_map.clone());

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Array(mir::ElementLayout {
                        element,
                        stride,
                        count,
                    }),
                    representation,
                    niche,
                    size,
                    alignment: element_alignment,
                    trace_map,
                })
            }

            // structs pack their named fields largest alignment first
            mir::Type::Struct { fields, .. } => {
                let mut components = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = self.tree.get(field).clone();
                    components.push((field.name, field.ty));
                }
                let aggregate = Aggregate::new(&components, self)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Struct(mir::StructLayout {
                        fields: aggregate.fields,
                    }),
                    representation: aggregate.representation,
                    niche: aggregate.niche,
                    size: aggregate.size,
                    alignment: aggregate.alignment,
                    trace_map: aggregate.trace_map,
                })
            }

            // tuples pack their elements the same way
            mir::Type::Tuple { elements, .. } => {
                let components: Vec<_> = elements.iter().map(|element| (None, *element)).collect();
                let aggregate = Aggregate::new(&components, self)?;

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tuple(mir::TupleLayout {
                        elements: aggregate.fields,
                    }),
                    representation: aggregate.representation,
                    niche: aggregate.niche,
                    size: aggregate.size,
                    alignment: aggregate.alignment,
                    trace_map: aggregate.trace_map,
                })
            }

            // newtypes store transparently as their inner type
            mir::Type::Newtype { inner, .. } => {
                let backing = self.layout_type(inner)?;
                let layout = self.layouts.layout(backing);
                let trace_map = layout.trace_map.clone();

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Newtype(mir::NewtypeLayout {
                        backing_type: inner,
                        backing_layout: backing,
                    }),
                    representation: layout.representation,
                    niche: layout.niche,
                    size: layout.size,
                    alignment: layout.alignment,
                    trace_map,
                })
            }

            // variants pack their widest payload behind a direct discriminant
            mir::Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                let variant = Variant::new(discriminant, &cases, self)?;

                variant.layout(self.module)
            }

            // vectors store fixed scalar lanes inline
            mir::Type::Vector { element, lanes, .. } => {
                let element_layout = self.layout_type(element)?;
                let element_layout = self.layouts.layout(element_layout);
                let stride = element_layout
                    .size
                    .next_multiple_of(element_layout.alignment);
                let size = stride
                    .checked_mul(lanes)
                    .ok_or_else(|| self.unsupported("vector"))?;
                let mir::Representation::Scalar(element_scalar) = element_layout.representation
                else {
                    return Err(self.unsupported("vector element"));
                };
                let alignment = self.natural_alignment(size);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Vector(mir::ElementLayout {
                        element,
                        stride,
                        count: lanes,
                    }),
                    representation: mir::Representation::Vector(mir::Vector::new(
                        element_scalar,
                        lanes,
                    )),
                    niche: None,
                    size,
                    alignment,
                    trace_map: mir::TraceMap::Empty,
                })
            }

            // tensors carry one storage reference
            mir::Type::Tensor {
                kind,
                storage,
                element,
                shape,
                format,
                sharding,
                nullability,
                ..
            } => {
                let rank = u32::try_from(shape.len()).map_err(|_| self.unsupported("tensor"))?;
                let reference = self.pointer_scalar(nullability);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Tensor(mir::TensorLayout {
                        element,
                        format,
                        sharding,
                        rank,
                    }),
                    representation: mir::Representation::Scalar(reference),
                    niche: reference.niche(0),
                    size: self.pointer_bytes(),
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::reference(kind, storage),
                })
            }

            // tensor views store a base, offset, dimensions, and strides
            mir::Type::TensorView {
                kind,
                storage,
                element,
                shape,
                format,
                sharding,
                nullability,
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
                let reference = self.pointer_scalar(nullability);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::TensorView(mir::TensorViewLayout {
                        element,
                        format,
                        sharding,
                        rank,
                    }),
                    representation: mir::Representation::Memory,
                    niche: reference.niche(0),
                    size,
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::reference(kind, storage),
                })
            }

            // dynamic values store one erased payload reference and dispatch table id
            mir::Type::Dynamic {
                kind,
                storage,
                nullability,
                ..
            } => {
                let payload = self.pointer_scalar(nullability);
                let table = mir::Scalar::new(mir::Primitive::Integer { width: 32 });
                let representation = mir::Representation::ScalarPair([
                    mir::ScalarField::new(payload, 0),
                    mir::ScalarField::new(table, self.pointer_bytes()),
                ]);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Dynamic,
                    representation,
                    niche: payload.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::reference(kind, storage),
                })
            }

            // closures store a function identity and erased environment reference
            mir::Type::Function {
                kind,
                storage,
                nullability,
                ..
            } => {
                let environment_offset = self.pointer_bytes();
                let environment_trace = mir::TraceMap::reference(kind, storage);
                let function = self.function_scalar(nullability);
                let environment = mir::Scalar::new(mir::Primitive::Pointer {
                    width: self.target.pointer_bits(),
                });
                let representation = mir::Representation::ScalarPair([
                    mir::ScalarField::new(function, 0),
                    mir::ScalarField::new(environment, environment_offset),
                ]);

                Ok(mir::Layout {
                    shape: mir::LayoutShape::Function,
                    representation,
                    niche: function.niche(0),
                    size: self.pointer_bytes() * 2,
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::nested(environment_offset, environment_trace),
                })
            }

            // bare function identities occupy one target word
            mir::Type::FunctionPointer { .. } => {
                let scalar = self.function_scalar(mir::Nullability::None);

                Ok(mir::Layout::scalar(
                    scalar,
                    self.pointer_bytes(),
                    self.pointer_alignment(),
                ))
            }

            // transparent storage forms are handled before layout construction
            mir::Type::Atomic { .. }
            | mir::Type::Application { .. }
            | mir::Type::Uninit { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::Error
            | mir::Type::Never
            | mir::Type::FunctionSignature { .. } => Err(self.unsupported("type")),
        }
    }

    /// Return one computed layout by id.
    pub(super) fn layout(&self, id: mir::LayoutId) -> &mir::Layout {
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
    fn integer_layout(&self, width: u16, is_signed: bool) -> Result<mir::Layout, LayoutError> {
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

                return Ok(mir::Layout {
                    shape: mir::LayoutShape::Scalar,
                    representation: mir::Representation::Memory,
                    niche: None,
                    size,
                    alignment: self.pointer_alignment(),
                    trace_map: mir::TraceMap::Empty,
                });
            }
        };
        let primitive = mir::Primitive::Integer {
            width: physical_width,
        };

        // preserve the logical value range inside the legalized scalar
        let scalar = if width == physical_width {
            mir::Scalar::new(primitive)
        } else {
            let magnitude = 1u128 << (width - 1);
            let validity = if is_signed {
                let physical_mask = mir::Scalar::new(primitive).bit_mask();
                mir::Validity::new(physical_mask - magnitude + 1, magnitude - 1)
            } else {
                mir::Validity::new(0, (magnitude << 1) - 1)
            };

            mir::Scalar::with_validity(primitive, validity)
        };
        let size = u32::from(physical_width).div_ceil(8);

        Ok(mir::Layout::scalar(
            scalar,
            size,
            self.natural_alignment(size),
        ))
    }

    /// Return one pointer scalar with the permitted nullish sentinels.
    fn pointer_scalar(&self, nullability: mir::Nullability) -> mir::Scalar {
        let primitive = mir::Primitive::Pointer {
            width: self.target.pointer_bits(),
        };

        mir::Scalar::with_nullability(primitive, nullability)
    }

    /// Return one callable identity with the permitted nullish sentinels.
    fn function_scalar(&self, nullability: mir::Nullability) -> mir::Scalar {
        let primitive = mir::Primitive::Integer {
            width: self.target.pointer_bits(),
        };

        mir::Scalar::with_nullability(primitive, nullability)
    }

    /// Return one unsupported physical representation diagnostic.
    fn unsupported(&self, construct: &str) -> LayoutError {
        LayoutError::Unsupported {
            module: self.module,
            construct: format!("a layout for this {construct}"),
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
            LayoutError::InvalidNiche { offset } => Self::Internal {
                message: format!(
                    "variant niche at byte offset {offset} is absent from its representation"
                ),
            },
            LayoutError::Recursive { ty } => Self::Internal {
                message: format!("type {ty:?} is value-recursive without indirection"),
            },
        }
    }
}
