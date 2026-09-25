use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_serde::Reflect;

use crate::{
    Access, FunctionId, GlobalId, Lifetime, LocalId, Reference, Substitution, Tree, Type, TypeId,
    Value,
};

/// The root of a MIR place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PlaceOrigin {
    /// A function local.
    Local(LocalId),
    /// A module global.
    Global(GlobalId),
    /// Storage rooted in an SSA value.
    Value(Value),
}

/// A storage location selected by a root and projections.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Place {
    /// The root storage.
    pub origin: PlaceOrigin,
    /// The structural path from the root.
    pub path: Path,
}

impl Place {
    /// Create a place from one origin.
    pub fn new(origin: PlaceOrigin) -> Self {
        Self {
            origin,
            path: Path::root(),
        }
    }

    /// Create a place rooted in one local.
    pub fn local(local: LocalId) -> Self {
        Self::new(PlaceOrigin::Local(local))
    }

    /// Create a place rooted in one global.
    pub fn global(global: GlobalId) -> Self {
        Self::new(PlaceOrigin::Global(global))
    }

    /// Create a place rooted in one SSA value.
    pub fn value(value: Value) -> Self {
        Self::new(PlaceOrigin::Value(value))
    }

    /// Return this place with one extra projection.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.path.push(projection);

        self
    }

    /// Return this place with another path appended.
    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = self.path.with_path(path);

        self
    }

    /// Append one projection.
    pub fn push(&mut self, projection: Projection) {
        self.path.push(projection);
    }

    /// Return the type of the root storage.
    pub fn root_type(&self, function: FunctionId, tree: &Tree) -> Option<TypeId> {
        match self.origin {
            PlaceOrigin::Local(local) => Some(tree.get(local).ty),
            PlaceOrigin::Global(global) => Some(tree.get(global).ty),
            PlaceOrigin::Value(value) => tree.get(function).value_type(value),
        }
    }

    /// Return the value or sequence selected by this place.
    pub fn ty(&self, function: FunctionId, tree: &Tree) -> Option<PlaceType> {
        let root = PlaceType::Value(self.root_type(function, tree)?);

        self.path
            .projections
            .iter()
            .try_fold(root, |ty, projection| ty.project(projection, tree))
    }

    /// Extend this prefix of another place with its projections up to the given length.
    pub fn extend_to(&mut self, place: &Self, length: usize) {
        let projections = &place.path.projections[self.path.projections.len()..length];
        self.path.projections.extend_from_slice(projections);
    }

    /// Iterate the length and type of each prefix this place projects from, typing each once.
    pub fn prefix_types(
        &self,
        function: FunctionId,
        tree: &Tree,
    ) -> impl Iterator<Item = (usize, PlaceType)> {
        let root = self
            .root_type(function, tree)
            .unwrap_or_else(|| unreachable!("a place has no root type"));

        // yield each prefix type before projecting it further
        self.path.projections.iter().enumerate().scan(
            PlaceType::Value(root),
            move |ty, (length, projection)| {
                let prefix = *ty;
                *ty = ty.project(projection, tree).unwrap_or_else(|| {
                    unreachable!("a place projects {projection:?} out of {prefix:?}")
                });

                Some((length, prefix))
            },
        )
    }

    /// Iterate the prefix length and reference type of each dereference this place follows.
    pub fn dereferences<'t>(
        &self,
        function: FunctionId,
        tree: &'t Tree,
    ) -> impl Iterator<Item = (usize, &'t Type)> {
        self.prefix_types(function, tree)
            .filter(|(length, _)| self.path.projections[*length] == Projection::Deref)
            .map(move |(length, ty)| {
                let PlaceType::Value(reference) = ty else {
                    unreachable!("a place dereferences a non-value");
                };

                (length, tree.type_definition(tree.storage_type(reference)))
            })
    }

    /// Return the last reference type traversed by this place.
    pub fn reference_type(&self, function: FunctionId, tree: &Tree) -> Option<TypeId> {
        let mut ty = PlaceType::Value(self.root_type(function, tree)?);
        let mut reference = None;

        // preserve the descriptor type before selecting its referent
        for projection in &self.path.projections {
            if *projection == Projection::Deref {
                let PlaceType::Value(ty) = ty else {
                    return None;
                };
                reference = Some(tree.storage_type(ty));
            }
            ty = ty.project(projection, tree)?;
        }

        reference
    }

    /// Return whether the address depends only on SSA values and fixed storage.
    pub fn is_stable(&self) -> bool {
        let projections = &self.path.projections;
        if let PlaceOrigin::Value(_) = self.origin
            && projections.first() == Some(&Projection::Deref)
        {
            !projections[1..].contains(&Projection::Deref)
        } else {
            !projections.contains(&Projection::Deref)
        }
    }

    /// Return the SSA values used by this place.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        let mut values = SmallVec::new();
        if let PlaceOrigin::Value(value) = self.origin {
            values.push(value);
        }

        // collect dynamic projection operands
        for projection in &self.path.projections {
            match projection {
                Projection::Index { index } => values.push(*index),
                Projection::Slice { start, length } => values.extend([*start, *length]),
                _ => {}
            }
        }

        values
    }

    /// Map the root value and dynamic projection operands.
    pub fn map_values(&mut self, mut map: impl FnMut(Value) -> Value) {
        if let PlaceOrigin::Value(value) = &mut self.origin {
            *value = map(*value);
        }

        self.path.map_values(map);
    }

    /// Return whether this place contains another place.
    pub fn contains(&self, other: &Self) -> bool {
        self.origin == other.origin && self.path.contains(&other.path)
    }
}

/// The value or unsized referent selected by a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceType {
    /// One sized value.
    Value(TypeId),
    /// The unsized elements or referent one slice, array, dynamic, or function type selects.
    Referent(TypeId),
}

impl PlaceType {
    /// Return the element type of a slice or fixed array referent.
    pub fn element(self, tree: &Tree) -> Option<TypeId> {
        match self {
            Self::Referent(descriptor) => match tree.type_definition(descriptor) {
                Type::Slice { element, .. } | Type::FixedArray { element, .. } => Some(*element),
                _ => None,
            },
            Self::Value(_) => None,
        }
    }

    /// Select and substitute one component.
    pub fn project(self, projection: &Projection, tree: &Tree) -> Option<Self> {
        match self {
            Self::Referent(_) => match projection {
                Projection::Element { .. } | Projection::Index { .. } | Projection::Elements => {
                    self.element(tree).map(Self::Value)
                }
                Projection::Slice { .. } => self.element(tree).map(|_| self),
                _ => None,
            },
            Self::Value(ty) => {
                // dereference a newtype through the reference it wraps
                if *projection == Projection::Deref
                    && matches!(tree.type_definition(ty), Type::Newtype { .. })
                {
                    return Self::Value(tree.storage_type(ty)).project(projection, tree);
                }

                // read the declared representation and retain its applied arguments
                let (ty, arguments) = match tree.get(ty) {
                    Type::Application { base, arguments } => (*base, arguments.clone()),
                    _ => (ty, Vec::new()),
                };
                let descriptor = ty;
                let ty = tree.type_definition(ty);

                // instantiate modified storage before projecting through it
                if let Type::Uninit { value } | Type::ManuallyDrop { value } = ty {
                    let value = *value;
                    let value = Substitution::new(tree, &arguments).ty(value);

                    return Self::Value(value).project(projection, tree);
                }

                let selected = match (projection, ty) {
                    (
                        Projection::Deref,
                        Type::Reference { pointee, .. } | Type::Pointer { pointee, .. },
                    ) => Some(Self::Value(*pointee)),
                    (
                        Projection::Deref,
                        Type::Slice { .. } | Type::Dynamic { .. } | Type::Function { .. },
                    ) => Some(Self::Referent(descriptor)),
                    (Projection::Field { index }, ty) => {
                        ty.field_type(*index, tree).map(Self::Value)
                    }
                    (Projection::Variant { case }, Type::Variant { cases, .. }) => {
                        cases.get(*case as usize).map(|case| Self::Value(case.ty))
                    }
                    (
                        Projection::Elements
                        | Projection::Element { .. }
                        | Projection::Index { .. },
                        Type::FixedArray { element, .. } | Type::Vector { element, .. },
                    ) => Some(Self::Value(*element)),
                    (Projection::Slice { .. }, Type::FixedArray { .. }) => {
                        Some(Self::Referent(descriptor))
                    }
                    _ => None,
                }?;

                // substitute only the selected field or element
                let mut substitution = Substitution::new(tree, &arguments);
                let selected = match selected {
                    Self::Value(ty) => Self::Value(substitution.ty(ty)),
                    Self::Referent(ty) => Self::Referent(substitution.ty(ty)),
                };

                Some(selected)
            }
        }
    }
}

/// One projection in a MIR path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Projection {
    /// Any element of a repeated type.
    Elements,
    /// A fixed concrete field projection.
    Field {
        /// The zero-based field index.
        index: u32,
    },
    /// A fixed element projection.
    Element {
        /// The zero-based element index.
        index: u32,
    },
    /// A runtime element projection.
    Index {
        /// The runtime index value.
        index: Value,
    },
    /// One statically selected variant case.
    Variant {
        /// The zero-based case index.
        case: u32,
    },
    /// A runtime slice projection.
    Slice {
        /// The runtime start index value.
        start: Value,
        /// The runtime length value.
        length: Value,
    },
    /// The referent of a reference.
    Deref,
}

impl Projection {
    /// Return whether this projection includes every selection of another projection.
    pub fn contains(&self, other: &Self) -> bool {
        self == other
            || matches!(
                (self, other),
                (
                    Self::Elements,
                    Self::Element { .. } | Self::Index { .. } | Self::Slice { .. }
                )
            )
    }

    /// Return whether two projections can select the same component.
    pub fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Field { index: left }, Self::Field { index: right })
            | (Self::Element { index: left }, Self::Element { index: right }) => left == right,
            (Self::Variant { case: left }, Self::Variant { case: right }) => left == right,
            (Self::Deref, Self::Deref) => true,
            (
                Self::Elements | Self::Element { .. } | Self::Index { .. } | Self::Slice { .. },
                Self::Elements | Self::Element { .. } | Self::Index { .. } | Self::Slice { .. },
            ) => true,
            _ => false,
        }
    }
}

/// A rootless path through a MIR value or type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub struct Path {
    /// Projections from the root value.
    pub projections: Vec<Projection>,
}

impl Path {
    /// Create a root path.
    #[inline]
    pub fn root() -> Self {
        Self::default()
    }

    /// Append one projection.
    #[inline]
    pub fn push(&mut self, projection: Projection) {
        self.projections.push(projection);
    }

    /// Return this path with one extra projection.
    #[inline]
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.push(projection);

        self
    }

    /// Return this path with another path appended.
    #[inline]
    pub fn with_path(mut self, path: &Path) -> Self {
        self.projections.extend(path.projections.iter().cloned());

        self
    }

    /// Return the first projection, absent at the root.
    pub fn first(&self) -> Option<&Projection> {
        self.projections.first()
    }

    /// Return whether this path is rooted at the value itself.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.projections.is_empty()
    }

    /// Return whether this path contains another path.
    pub fn contains(&self, other: &Self) -> bool {
        self.projections.len() <= other.projections.len()
            && self
                .projections
                .iter()
                .zip(&other.projections)
                .all(|(left, right)| left.contains(right))
    }

    /// Return whether the paths can select overlapping storage.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.projections
            .iter()
            .zip(&other.projections)
            .all(|(left, right)| left.overlaps(right))
    }

    /// Remove one matching structural prefix from this path.
    pub fn strip_prefix(&self, prefix: &Self) -> Option<Self> {
        if prefix.projections.len() > self.projections.len() || !prefix.overlaps(self) {
            return None;
        }
        let projections = self.projections[prefix.projections.len()..].to_vec();

        Some(Self { projections })
    }

    /// Map each value reference inside this path.
    pub fn map_values(&mut self, mut map: impl FnMut(Value) -> Value) {
        for projection in &mut self.projections {
            match projection {
                Projection::Index { index } => *index = map(*index),
                Projection::Slice { start, length } => {
                    *start = map(*start);
                    *length = map(*length);
                }
                Projection::Elements
                | Projection::Field { .. }
                | Projection::Element { .. }
                | Projection::Variant { .. }
                | Projection::Deref => {}
            }
        }
    }

    /// Replace value references inside this path.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        self.map_values(|value| if value == from { to } else { value });
    }
}

/// One reference-like component in a type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BorrowedPath {
    /// The path to the component.
    pub path: Path,
    /// The lifetime carried by the component.
    pub lifetime: Lifetime,
    /// Access granted by the component.
    pub access: Access,
    /// The reference kind of the component.
    pub kind: Reference,
}
