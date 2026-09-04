use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One lifetime slot in a MIR lifetime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LifetimeSlot(pub u32);

/// One declared lifetime parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LifetimeParameter {
    /// The source or generated parameter name.
    pub name: Option<StringId>,
    /// The declared slots this parameter outlives.
    pub outlives: Vec<LifetimeSlot>,
}

impl LifetimeParameter {
    /// Create a lifetime parameter with an optional name.
    pub fn new(name: Option<StringId>) -> Self {
        Self {
            name,
            outlives: Vec::new(),
        }
    }

    /// Create a lifetime parameter with declared outlives slots.
    pub fn with_outlives(name: Option<StringId>, outlives: Vec<LifetimeSlot>) -> Self {
        Self { name, outlives }
    }
}

/// One term in a MIR lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LifetimeTerm {
    /// Global or static storage.
    Static,
    /// Storage owned by the current activation.
    Frame,
    /// Managed storage, alive while reachable, its handles held live across parks.
    Managed,
    /// A lifetime slot in the current lifetime environment.
    Slot(LifetimeSlot),
}

/// The boundary lifetime for an escaping borrowed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub struct Lifetime {
    /// Terms the borrowed value may depend on.
    pub terms: Vec<LifetimeTerm>,
}

impl Lifetime {
    /// Create an empty lifetime.
    pub fn empty() -> Self {
        Self { terms: Vec::new() }
    }

    /// Create a lifetime from terms.
    pub fn new(terms: impl IntoIterator<Item = LifetimeTerm>) -> Self {
        let mut unique = Vec::new();

        for term in terms {
            if !unique.contains(&term) {
                unique.push(term);
            }
        }

        Self { terms: unique }
    }

    /// Create a lifetime rooted in static storage.
    pub fn static_storage() -> Self {
        Self::new([LifetimeTerm::Static])
    }

    /// Create a lifetime rooted in the current activation.
    pub fn frame() -> Self {
        Self::new([LifetimeTerm::Frame])
    }

    /// Create the managed storage lifetime.
    pub fn managed() -> Self {
        Self::new([LifetimeTerm::Managed])
    }

    /// Create a lifetime bound to one slot.
    pub fn slot(index: u32) -> Self {
        Self::new([LifetimeTerm::Slot(LifetimeSlot(index))])
    }

    /// Create a lifetime bound to multiple slots.
    pub fn slot_set(indices: impl IntoIterator<Item = u32>) -> Self {
        Self::new(
            indices
                .into_iter()
                .map(|index| LifetimeTerm::Slot(LifetimeSlot(index))),
        )
    }

    /// Return whether this lifetime has no escaping borrow source.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Return whether this lifetime includes static storage.
    pub fn includes_static(&self) -> bool {
        self.terms.contains(&LifetimeTerm::Static)
    }

    /// Return whether this lifetime includes the current activation.
    pub fn includes_frame(&self) -> bool {
        self.terms.contains(&LifetimeTerm::Frame)
    }

    /// Return whether this lifetime is exactly static storage.
    pub fn is_static(&self) -> bool {
        self.terms.as_slice() == [LifetimeTerm::Static]
    }

    /// Return whether this lifetime is exactly the current activation.
    pub fn is_frame(&self) -> bool {
        self.terms.as_slice() == [LifetimeTerm::Frame]
    }

    /// Return whether this lifetime includes a slot.
    pub fn includes_slot(&self, index: u32) -> bool {
        self.terms
            .contains(&LifetimeTerm::Slot(LifetimeSlot(index)))
    }

    /// Return lifetime slot indices.
    pub fn slot_indices(&self) -> impl Iterator<Item = u32> + '_ {
        self.terms.iter().filter_map(|term| match term {
            LifetimeTerm::Slot(index) => Some(index.0),
            LifetimeTerm::Static | LifetimeTerm::Frame | LifetimeTerm::Managed => None,
        })
    }

    /// Return whether every source region outlives this lifetime.
    pub fn accepts(&self, source: &Self, parameters: &[LifetimeParameter]) -> bool {
        !source.is_empty()
            && source.terms.iter().all(|source| {
                self.terms
                    .iter()
                    .any(|required| Self::term_outlives(*source, *required, parameters))
            })
    }

    /// Return whether one term lives at least as long as another term.
    fn term_outlives(
        longer: LifetimeTerm,
        shorter: LifetimeTerm,
        parameters: &[LifetimeParameter],
    ) -> bool {
        let mut visited = vec![false; parameters.len()];

        Self::term_outlives_inner(longer, shorter, parameters, &mut visited)
    }

    /// Traverse declared lifetime bounds.
    fn term_outlives_inner(
        longer: LifetimeTerm,
        shorter: LifetimeTerm,
        parameters: &[LifetimeParameter],
        visited: &mut [bool],
    ) -> bool {
        match (longer, shorter) {
            (LifetimeTerm::Static, _) => true,
            (LifetimeTerm::Frame, LifetimeTerm::Frame) => true,
            (LifetimeTerm::Managed, LifetimeTerm::Managed | LifetimeTerm::Frame) => true,
            (LifetimeTerm::Slot(_), LifetimeTerm::Frame) => true,
            (LifetimeTerm::Slot(left), LifetimeTerm::Slot(right)) if left == right => true,
            (LifetimeTerm::Slot(left), LifetimeTerm::Slot(right)) => {
                let index = left.0 as usize;
                let is_visited = visited.get(index).copied().unwrap_or(false);
                if is_visited {
                    return false;
                }
                let Some(parameter) = parameters.get(index) else {
                    return false;
                };
                visited[index] = true;

                let is_outlived = parameter.outlives.iter().copied().any(|shorter| {
                    Self::term_outlives_inner(
                        LifetimeTerm::Slot(shorter),
                        LifetimeTerm::Slot(right),
                        parameters,
                        visited,
                    )
                });
                visited[index] = false;

                is_outlived
            }
            _ => false,
        }
    }
}
