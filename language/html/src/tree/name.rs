use destack_core::{StringId, StringPool};
use serde::{Deserialize, Serialize};

/// One HTML name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Name {
    /// The optional qualified prefix.
    pub prefix: Option<StringId>,
    /// The resolved namespace.
    pub namespace: Namespace,
    /// The local name.
    pub local: StringId,
}

impl Name {
    /// Return the local name as one string slice.
    pub fn local_eq(&self, strings: &StringPool, expected: &str) -> bool {
        strings.get(self.local) == expected
    }

    /// Render this name as one qualified string.
    pub fn render(&self, strings: &StringPool) -> String {
        let local = strings.get(self.local);

        if let Some(prefix) = self.prefix {
            let prefix = strings.get(prefix);

            return format!("{prefix}:{local}");
        }

        local.to_string()
    }
}

/// One HTML namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Namespace {
    /// The HTML namespace.
    Html,
    /// The SVG namespace.
    Svg,
    /// The MathML namespace.
    MathMl,
    /// The XML namespace.
    Xml,
    /// The XMLNS namespace.
    XmlNs,
    /// The XLINK namespace.
    XLink,
    /// One other resolved namespace URI.
    Other(String),
}
