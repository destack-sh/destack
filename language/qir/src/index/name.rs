use serde::{Deserialize, Serialize};

/// Searchable source name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Name {
    /// Textual name.
    String(String),
    /// Positional name.
    Index(u64),
}

impl Name {
    /// Return the searchable display text for this name.
    pub fn text(&self) -> String {
        match self {
            Self::String(text) => text.clone(),
            Self::Index(index) => index.to_string(),
        }
    }

    /// Return whether this name matches one lowercase query.
    pub fn matches_lowercase_query(&self, query: &str) -> bool {
        let text = self.text();

        query.is_empty() || text.to_lowercase().contains(query)
    }
}

impl From<String> for Name {
    /// Convert one string into a textual name.
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Name {
    /// Convert one string slice into a textual name.
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
