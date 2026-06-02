use destack_core::StringId;
use serde::{Deserialize, Serialize};

/// One lifetime slot in a MIR lifetime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LifetimeSlot(pub u32);

/// One declared lifetime parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LifetimeParameter {
    /// The source or generated parameter name.
    pub name: Option<StringId>,
}

impl LifetimeParameter {
    /// Create a lifetime parameter with an optional name.
    pub fn new(name: Option<StringId>) -> Self {
        Self { name }
    }
}

/// One term in a MIR lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifetimeTerm {
    /// Global or static storage.
    Static,
    /// A lifetime slot in the current lifetime environment.
    Slot(LifetimeSlot),
}

/// The boundary lifetime for an escaping borrowed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

    /// Return whether this lifetime is exactly static storage.
    pub fn is_static(&self) -> bool {
        self.terms.as_slice() == [LifetimeTerm::Static]
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
            LifetimeTerm::Static => None,
        })
    }
}
