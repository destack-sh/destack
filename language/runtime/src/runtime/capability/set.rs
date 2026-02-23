use rustc_hash::FxHashSet;

use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId};

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

    /// Create one capability set from one iterator of capability identifiers.
    pub fn from_ids(ids: impl IntoIterator<Item = PlatformCapabilityId>) -> Self {
        ids.into_iter().collect()
    }

    /// Create one capability set from one iterator of capability kinds.
    pub fn from_capabilities(capabilities: impl IntoIterator<Item = PlatformCapability>) -> Self {
        let capability_ids = capabilities.into_iter().map(PlatformCapability::id);
        Self::from_ids(capability_ids)
    }

    /// Insert one capability identifier.
    pub fn insert_id(&mut self, capability: PlatformCapabilityId) -> bool {
        self.identifiers.insert(capability)
    }

    /// Insert one capability name.
    pub fn insert_name(&mut self, capability_name: &str) -> bool {
        self.insert_id(PlatformCapabilityId::from_name(capability_name))
    }

    /// Insert one capability kind.
    pub fn insert_capability(&mut self, capability: PlatformCapability) -> bool {
        self.insert_id(capability.id())
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

    /// Extend this set from one iterator of capability kinds.
    pub fn extend_capabilities(
        &mut self,
        capabilities: impl IntoIterator<Item = PlatformCapability>,
    ) {
        for capability in capabilities {
            self.insert_capability(capability);
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

    /// Return whether this set contains one capability kind.
    pub fn contains_capability(&self, capability: PlatformCapability) -> bool {
        self.contains_id(capability.id())
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

impl FromIterator<PlatformCapability> for PlatformCapabilitySet {
    fn from_iter<T: IntoIterator<Item = PlatformCapability>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend_capabilities(iter);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::PlatformCapabilitySet;
    use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId};

    #[test]
    fn test_from_capabilities_contains_capability_kinds_and_ids() {
        let set = PlatformCapabilitySet::from_capabilities([
            PlatformCapability::FsRead,
            PlatformCapability::NetConnect,
        ]);

        assert!(set.contains_capability(PlatformCapability::FsRead));
        assert!(set.contains_capability(PlatformCapability::NetConnect));
        assert!(set.contains_id(PlatformCapabilityId::from_name("fs.read")));
        assert!(set.contains_id(PlatformCapabilityId::from_name("net.connect")));
    }

    #[test]
    fn test_extend_capabilities_merges_into_existing_set() {
        let mut set = PlatformCapabilitySet::from_names(["gpu.device"]);
        set.extend_capabilities([PlatformCapability::GpuQueue, PlatformCapability::GpuPresent]);

        assert!(set.contains_name("gpu.device"));
        assert!(set.contains_capability(PlatformCapability::GpuQueue));
        assert!(set.contains_capability(PlatformCapability::GpuPresent));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_insert_capability_deduplicates_existing_capability() {
        let mut set = PlatformCapabilitySet::new();
        let first_insert = set.insert_capability(PlatformCapability::IoPoll);
        let second_insert = set.insert_capability(PlatformCapability::IoPoll);

        assert!(first_insert);
        assert!(!second_insert);
        assert_eq!(set.len(), 1);
    }
}
