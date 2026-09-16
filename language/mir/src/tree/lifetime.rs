use destack_core::{SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// One region parameter identified relative to its enclosing binder.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
#[repr(C)]
pub struct RegionBound {
    /// The number of enclosing region binders before the declaring binder.
    pub depth: u32,
    /// The parameter index within the declaring binder.
    pub index: u32,
}

impl RegionBound {
    /// Refer to one parameter of the nearest region binder.
    pub const fn new(index: u32) -> Self {
        Self { depth: 0, index }
    }
}

/// One declared lifetime parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LifetimeParameter {
    /// The source or generated parameter name.
    pub name: Option<StringId>,
    /// The lifetime extents this parameter outlives.
    pub outlives: Lifetime,
}

impl LifetimeParameter {
    /// Create a lifetime parameter with an optional name.
    pub fn new(name: Option<StringId>) -> Self {
        Self {
            name,
            outlives: Lifetime::empty(),
        }
    }

    /// Create a lifetime parameter with declared outlives bounds.
    pub fn with_outlives(name: Option<StringId>, outlives: Lifetime) -> Self {
        Self { name, outlives }
    }
}

/// One extent a MIR lifetime ranges over.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum Extent {
    /// Global or static storage.
    Static,
    /// Storage owned by the current activation.
    Frame,
    /// Managed storage, alive while reachable, its handles held live across parks.
    Managed,
    /// A lifetime parameter in an enclosing binder.
    Bound(RegionBound),
    /// A region parameter of the enclosing type declaration, by generic index.
    Parameter(u32),
}

/// The lifetime one escaping borrowed value stays within.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub struct Lifetime {
    /// The extents the borrowed value may depend on.
    pub extents: Vec<Extent>,
}

impl Lifetime {
    /// Create an empty lifetime.
    pub fn empty() -> Self {
        Self {
            extents: Vec::new(),
        }
    }

    /// Create a lifetime from extents.
    pub fn new(extents: impl IntoIterator<Item = Extent>) -> Self {
        let mut unique = Vec::new();
        for extent in extents {
            if !unique.contains(&extent) {
                unique.push(extent);
            }
        }

        Self { extents: unique }
    }

    /// Create a lifetime rooted in static storage.
    pub fn static_storage() -> Self {
        Self::new([Extent::Static])
    }

    /// Create a lifetime rooted in the current activation.
    pub fn frame() -> Self {
        Self::new([Extent::Frame])
    }

    /// Create the managed storage lifetime.
    pub fn managed() -> Self {
        Self::new([Extent::Managed])
    }

    /// Create a lifetime bound to one parameter in the nearest binder.
    pub fn bound(index: u32) -> Self {
        Self::new([Extent::Bound(RegionBound::new(index))])
    }

    /// Create a lifetime bound to parameters in the nearest binder.
    pub fn bound_set(indices: impl IntoIterator<Item = u32>) -> Self {
        Self::new(
            indices
                .into_iter()
                .map(|index| Extent::Bound(RegionBound::new(index))),
        )
    }

    /// Return whether this lifetime has no escaping borrow source.
    pub fn is_empty(&self) -> bool {
        self.extents.is_empty()
    }

    /// Return whether this lifetime includes static storage.
    pub fn includes_static(&self) -> bool {
        self.extents.contains(&Extent::Static)
    }

    /// Return whether this lifetime includes managed storage.
    pub fn includes_managed(&self) -> bool {
        self.extents.contains(&Extent::Managed)
    }

    /// Return whether this lifetime includes the current activation.
    pub fn includes_frame(&self) -> bool {
        self.extents.contains(&Extent::Frame)
    }

    /// Return whether this lifetime is exactly static storage.
    pub fn is_static(&self) -> bool {
        self.extents.as_slice() == [Extent::Static]
    }

    /// Return whether this lifetime is exactly the current activation.
    pub fn is_frame(&self) -> bool {
        self.extents.as_slice() == [Extent::Frame]
    }

    /// Return whether this lifetime includes a parameter of the nearest binder.
    pub fn includes_bound(&self, index: u32) -> bool {
        self.extents
            .contains(&Extent::Bound(RegionBound::new(index)))
    }

    /// Return parameter indices referenced in the nearest lifetime binder.
    pub fn bound_indices(&self) -> impl Iterator<Item = u32> + '_ {
        self.extents.iter().filter_map(|term| match term {
            Extent::Bound(bound) if bound.depth == 0 => Some(bound.index),
            Extent::Bound(_) => None,
            Extent::Static | Extent::Frame | Extent::Managed | Extent::Parameter(_) => None,
        })
    }

    /// Return whether every source region outlives this lifetime.
    pub fn accepts(&self, source: &Self, parameters: &[LifetimeParameter]) -> bool {
        !source.is_empty()
            && source.extents.iter().all(|source| {
                self.extents
                    .iter()
                    .any(|required| source.outlives(*required, parameters))
            })
    }
}

impl Extent {
    /// Return whether this extent lives at least as long as another extent.
    pub fn outlives(self, shorter: Self, parameters: &[LifetimeParameter]) -> bool {
        // compare closed extents and identical parameters directly
        if self == shorter || self == Self::Static {
            return true;
        }
        match (self, shorter) {
            (Self::Managed, Self::Frame | Self::Managed | Self::Bound(_) | Self::Parameter(_)) => {
                true
            }
            (Self::Bound(_) | Self::Parameter(_), Self::Frame) => true,
            (Self::Bound(bound), _) if bound.depth == 0 => {
                let mut visited = SmallVec::<[bool; 8]>::new();
                visited.resize(parameters.len(), false);

                Self::bound_outlives(bound.index, shorter, parameters, &mut visited)
            }
            _ => false,
        }
    }

    /// Traverse each declared lifetime bound at most once.
    fn bound_outlives(
        index: u32,
        shorter: Self,
        parameters: &[LifetimeParameter],
        visited: &mut [bool],
    ) -> bool {
        let index = index as usize;
        if visited[index] {
            return false;
        }
        visited[index] = true;

        parameters[index]
            .outlives
            .extents
            .iter()
            .copied()
            .any(|extent| {
                if extent == shorter {
                    true
                } else if let Self::Bound(bound) = extent
                    && bound.depth == 0
                {
                    Self::bound_outlives(bound.index, shorter, parameters, visited)
                } else {
                    extent.outlives(shorter, parameters)
                }
            })
    }
}
