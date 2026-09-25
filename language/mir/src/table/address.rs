use smallvec::SmallVec;

use crate::{
    FunctionId, Layout, LayoutError, LayoutShape, LayoutTable, Place, PlaceType, Projection,
    Representation, Tree, Type, TypeId, Value,
};

/// One address computation from a place's root toward its selected storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressStep {
    /// Load the descriptor stored at the address and continue at its referent.
    Follow(Descriptor),
    /// Advance by a constant byte offset.
    Offset(u64),
    /// Advance by a runtime index scaled by an element stride.
    Index {
        /// The index value.
        index: Value,
        /// The element byte stride.
        stride: u32,
        /// The selected length, when the step selects a slice.
        length: Option<Value>,
    },
}

/// The words of one reference-like descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Descriptor {
    /// The storage type of the descriptor.
    pub ty: TypeId,
    /// The memory the address word points into.
    pub kind: AddressKind,
    /// The byte offset of the address word.
    pub address: u32,
    /// The byte offset of the second word, like a slice length, when the descriptor has one.
    pub metadata: Option<u32>,
}

/// The memory one address points into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressKind {
    /// A world offset, added to the memory base for machine access.
    Reference,
    /// A process-local machine pointer.
    Pointer,
}

impl Descriptor {
    /// Return the descriptor words of one reference-like type.
    pub fn new(ty: TypeId, tree: &Tree, layouts: &LayoutTable) -> Result<Self, LayoutError> {
        let ty = tree.storage_type(ty);
        let invalid = || LayoutError::Unsupported {
            construct: format!("a descriptor of the non-reference type {ty:?}"),
        };

        // select the memory the address word points into
        let kind = match tree.type_definition(ty) {
            Type::Pointer { .. } => AddressKind::Pointer,
            definition if definition.is_reference_representation() => AddressKind::Reference,
            _ => return Err(invalid()),
        };

        // locate the address word, the closure environment in the second word of a function
        let layout = layouts.type_layout(ty).ok_or(LayoutError::Missing { ty })?;
        let (address, metadata) = match (&layout.shape, layout.representation) {
            (LayoutShape::Function, Representation::ScalarPair([function, environment])) => {
                (environment.offset, Some(function.offset))
            }
            (_, Representation::ScalarPair([address, metadata])) => {
                (address.offset, Some(metadata.offset))
            }
            (_, Representation::Scalar(_)) => (0, None),
            _ => return Err(invalid()),
        };

        Ok(Self {
            ty,
            kind,
            address,
            metadata,
        })
    }
}

impl LayoutTable {
    /// Resolve the address steps of one place.
    pub fn address_steps(
        &self,
        place: &Place,
        function: FunctionId,
        tree: &Tree,
    ) -> Result<SmallVec<[AddressStep; 4]>, LayoutError> {
        let invalid = || LayoutError::Unsupported {
            construct: format!("the memory place {place:?}"),
        };
        let root = place.root_type(function, tree).ok_or_else(invalid)?;
        let mut ty = PlaceType::Value(root);
        let mut steps = SmallVec::new();

        // resolve each projection against the layout it selects from
        for projection in &place.path.projections {
            let selected = ty.project(projection, tree).ok_or_else(invalid)?;
            let step = match (projection, ty) {
                // follow the descriptor stored at the address
                (Projection::Deref, PlaceType::Value(reference)) => {
                    AddressStep::Follow(Descriptor::new(reference, tree, self)?)
                }
                // select a newtype's backing value at byte zero
                (Projection::Field { index: 0 }, PlaceType::Value(aggregate))
                    if is_newtype(aggregate, tree) =>
                {
                    AddressStep::Offset(0)
                }
                // offset to one field or variant payload
                (Projection::Field { index }, PlaceType::Value(aggregate)) => {
                    let field = self
                        .storage_layout(aggregate, tree)?
                        .source_field(*index)
                        .ok_or_else(invalid)?;

                    AddressStep::Offset(u64::from(field.offset))
                }
                (Projection::Variant { case }, PlaceType::Value(variant)) => {
                    let LayoutShape::Variant(layout) = &self.storage_layout(variant, tree)?.shape
                    else {
                        return Err(invalid());
                    };
                    let case = layout.cases.get(*case as usize).ok_or_else(invalid)?;

                    AddressStep::Offset(u64::from(case.payload_offset))
                }
                // offset to one fixed element or scale one runtime index
                (Projection::Element { index }, _) => {
                    let stride = self.stride(selected, tree)?;

                    AddressStep::Offset(u64::from(stride) * u64::from(*index))
                }
                (Projection::Index { index }, _) => AddressStep::Index {
                    index: *index,
                    stride: self.stride(selected, tree)?,
                    length: None,
                },
                (Projection::Slice { start, length }, _) => AddressStep::Index {
                    index: *start,
                    stride: self.stride(selected, tree)?,
                    length: Some(*length),
                },
                // reject abstract elements and value projections out of a referent
                _ => return Err(invalid()),
            };
            steps.push(step);
            ty = selected;
        }

        Ok(steps)
    }

    /// Return the layout of one type's initialized storage.
    fn storage_layout(&self, ty: TypeId, tree: &Tree) -> Result<&Layout, LayoutError> {
        let ty = tree.storage_type(ty);

        self.type_layout(ty).ok_or(LayoutError::Missing { ty })
    }

    /// Return the byte stride of the elements one indexed projection selects.
    fn stride(&self, selected: PlaceType, tree: &Tree) -> Result<u32, LayoutError> {
        let element = match selected {
            PlaceType::Value(element) => Some(element),
            referent => referent.element(tree),
        };
        let element = element.ok_or_else(|| LayoutError::Unsupported {
            construct: format!("an indexed projection of {selected:?}"),
        })?;
        let stride = self.storage_layout(element, tree)?.stride();

        u32::try_from(stride).map_err(|_| LayoutError::Unsupported {
            construct: format!("an element stride of {stride} bytes"),
        })
    }
}

/// Return whether one type is a newtype beneath its transparent storage forms.
fn is_newtype(ty: TypeId, tree: &Tree) -> bool {
    match tree.type_definition(ty) {
        Type::Newtype { .. } => true,
        Type::Uninit { value } | Type::ManuallyDrop { value } => is_newtype(*value, tree),
        _ => false,
    }
}
