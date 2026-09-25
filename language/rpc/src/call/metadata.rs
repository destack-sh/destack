use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// RPC metadata attached to one request or response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Metadata {
    /// Ordered metadata entries, including repeated names.
    entries: Vec<(String, Vec<u8>)>,
}

impl Metadata {
    /// Create empty RPC metadata.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append one metadata value.
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<Vec<u8>>) {
        self.entries.push((name.into(), value.into()));
    }

    /// Return every metadata entry in transmission order.
    pub fn entries(&self) -> &[(String, Vec<u8>)] {
        &self.entries
    }

    /// Return the first value with one exact name.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|(entry_name, _)| entry_name == name)
            .map(|(_, value)| value.as_slice())
    }

    /// Return whether the metadata is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
