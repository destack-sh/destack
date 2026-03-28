use serde::{Deserialize, Serialize};

/// One HTML name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Name {
    /// The optional qualified prefix.
    pub prefix: Option<String>,
    /// The resolved namespace.
    pub namespace: Namespace,
    /// The local name.
    pub local: String,
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
