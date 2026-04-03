use crate::{Name, Node, NodeType, StringId};
use serde::{Deserialize, Serialize};

/// One HTML attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    /// The attribute name.
    pub name: Name,
    /// The authored attribute name spelling when one exists.
    pub authored_name: Option<StringId>,
    /// The attribute value when one exists.
    pub value: Option<AttributeValue>,
}

impl Node for Attribute {
    const TYPE: NodeType = NodeType::Attribute;
}

/// One HTML attribute value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeValue {
    /// The normalized attribute value.
    pub value: String,
    /// The authored attribute value form.
    pub form: AttributeValueForm,
    /// The structured resource payload when this value owns one.
    pub resource: Option<AttributeResource>,
}

/// One HTML attribute value form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeValueForm {
    /// One double-quoted attribute value.
    DoubleQuoted,
    /// One single-quoted attribute value.
    SingleQuoted,
    /// One unquoted attribute value.
    Unquoted,
}

/// One structured HTML attribute resource value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeResource {
    /// One single rewriteable resource value.
    Resource(HtmlResource),
    /// One `srcset`-style multi-candidate value.
    SourceSet(SourceSetResource),
}

/// One single HTML resource value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HtmlResource {
    /// The resource role for this value.
    pub kind: HtmlResourceKind,
    /// The path portion of the authored value.
    pub path: String,
    /// The suffix portion of the authored value.
    pub suffix: String,
    /// Whether this value should stay untouched.
    pub is_external: bool,
}

impl HtmlResource {
    /// Create one HTML resource from one authored specifier.
    pub fn new(kind: HtmlResourceKind, specifier: &str) -> Self {
        let (path, suffix) = Self::split_specifier(specifier);

        Self {
            kind,
            path: path.to_string(),
            suffix: suffix.to_string(),
            is_external: Self::is_external_specifier(specifier),
        }
    }

    /// Return whether one authored specifier should stay untouched.
    pub(crate) fn is_external_specifier(specifier: &str) -> bool {
        let specifier = specifier.trim();

        if specifier.is_empty() {
            return true;
        }

        specifier.starts_with('#')
            || specifier.starts_with("http://")
            || specifier.starts_with("https://")
            || specifier.starts_with("//")
            || specifier.starts_with("data:")
    }

    /// Split one authored specifier into path and suffix.
    pub(crate) fn split_specifier(specifier: &str) -> (&str, &str) {
        let suffix_start = specifier.find(['?', '#']).unwrap_or(specifier.len());

        specifier.split_at(suffix_start)
    }
}

/// One HTML resource role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HtmlResourceKind {
    /// One module script reference.
    ModuleScript,
    /// One stylesheet reference.
    Stylesheet,
    /// One asset-like reference.
    Asset,
}

/// One `srcset`-style attribute resource value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSetResource {
    /// The candidate values in authored order.
    pub items: Vec<SourceSetItem>,
}

/// One `srcset` candidate value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSetItem {
    /// The exact candidate value written in source.
    pub value: String,
    /// The candidate path portion.
    pub path: String,
    /// The candidate suffix portion.
    pub suffix: String,
    /// The candidate descriptor suffix.
    pub descriptor: String,
    /// Whether this candidate should stay untouched.
    pub is_external: bool,
}

impl SourceSetItem {
    /// Create one source-set item from one authored candidate.
    pub fn new(value: &str, descriptor: &str) -> Self {
        let (path, suffix) = HtmlResource::split_specifier(value);

        Self {
            value: value.to_string(),
            path: path.to_string(),
            suffix: suffix.to_string(),
            descriptor: descriptor.to_string(),
            is_external: HtmlResource::is_external_specifier(value),
        }
    }
}
