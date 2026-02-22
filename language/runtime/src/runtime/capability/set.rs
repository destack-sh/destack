use rustc_hash::FxHashSet;

use crate::runtime::capability::PlatformCapabilityId;

/// Set of canonical platform capability identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformCapabilitySet {
    /// Stored capability identifiers.
    identifiers: FxHashSet<PlatformCapabilityId>,
}

impl PlatformCapabilitySet {
    /// Create one empty capability set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one capability set from one iterator of capability names.
    pub fn from_names<T>(names: impl IntoIterator<Item = T>) -> Self
    where
        T: AsRef<str>,
    {
        let mut set = Self::new();
        set.extend_names(names);

        set
    }

    /// Insert one capability identifier.
    pub fn insert_id(&mut self, capability: PlatformCapabilityId) -> bool {
        self.identifiers.insert(capability)
    }

    /// Insert one capability name.
    pub fn insert_name(&mut self, capability_name: &str) -> bool {
        self.insert_id(PlatformCapabilityId::from_name(capability_name))
    }

    /// Extend this set from one iterator of capability names.
    pub fn extend_names<T>(&mut self, names: impl IntoIterator<Item = T>)
    where
        T: AsRef<str>,
    {
        for name in names {
            self.insert_name(name.as_ref());
        }
    }

    /// Return whether this set contains one capability identifier.
    pub fn contains_id(&self, capability: PlatformCapabilityId) -> bool {
        self.identifiers.contains(&capability)
    }

    /// Return whether this set contains one capability name.
    pub fn contains_name(&self, capability_name: &str) -> bool {
        self.contains_id(PlatformCapabilityId::from_name(capability_name))
    }

    /// Return the number of capabilities in this set.
    pub fn len(&self) -> usize {
        self.identifiers.len()
    }

    /// Return whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.identifiers.is_empty()
    }

    /// Iterate the capability identifiers in this set.
    pub fn iter(&self) -> impl Iterator<Item = &PlatformCapabilityId> {
        self.identifiers.iter()
    }
}

impl FromIterator<PlatformCapabilityId> for PlatformCapabilitySet {
    fn from_iter<T: IntoIterator<Item = PlatformCapabilityId>>(iter: T) -> Self {
        let mut set = Self::new();
        set.identifiers.extend(iter);

        set
    }
}
