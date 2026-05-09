use serde::{Deserialize, Serialize};

/// A root that can keep an escaping borrowed value alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifetimeOrigin {
    /// Global or static storage.
    Static,
    /// A function parameter by index.
    Parameter(u32),
}

/// The boundary lifetime for an escaping borrowed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Lifetime {
    /// Origins the borrowed value may depend on.
    pub origins: Vec<LifetimeOrigin>,
}

impl Lifetime {
    /// Create an empty lifetime.
    pub fn empty() -> Self {
        Self {
            origins: Vec::new(),
        }
    }

    /// Create a lifetime from origins.
    pub fn new(origins: impl IntoIterator<Item = LifetimeOrigin>) -> Self {
        let mut unique = Vec::new();

        for origin in origins {
            if !unique.contains(&origin) {
                unique.push(origin);
            }
        }

        Self { origins: unique }
    }

    /// Create a lifetime rooted in static storage.
    pub fn static_storage() -> Self {
        Self::new([LifetimeOrigin::Static])
    }

    /// Create a lifetime bound to one parameter.
    pub fn parameter(index: u32) -> Self {
        Self::new([LifetimeOrigin::Parameter(index)])
    }

    /// Create a lifetime bound to multiple parameters.
    pub fn parameter_set(indices: impl IntoIterator<Item = u32>) -> Self {
        Self::new(indices.into_iter().map(LifetimeOrigin::Parameter))
    }

    /// Return whether this lifetime has no escaping borrow source.
    pub fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }

    /// Return whether this lifetime includes static storage.
    pub fn includes_static(&self) -> bool {
        self.origins.contains(&LifetimeOrigin::Static)
    }

    /// Return whether this lifetime is exactly static storage.
    pub fn is_static(&self) -> bool {
        self.origins.as_slice() == [LifetimeOrigin::Static]
    }

    /// Return whether this lifetime includes a parameter.
    pub fn includes_parameter(&self, index: u32) -> bool {
        self.origins.contains(&LifetimeOrigin::Parameter(index))
    }

    /// Return parameter origin indices.
    pub fn parameter_indices(&self) -> impl Iterator<Item = u32> + '_ {
        self.origins.iter().filter_map(|origin| match origin {
            LifetimeOrigin::Parameter(index) => Some(*index),
            LifetimeOrigin::Static => None,
        })
    }
}
