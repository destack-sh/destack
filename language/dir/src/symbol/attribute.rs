use serde::{Deserialize, Serialize};

use crate::StringId;

/// A semantic attribute attached to a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolAttribute {
    /// A host binding exported through the runtime binding ABI.
    Binding { name: Option<StringId> },
    /// An external host symbol name.
    Extern { name: Option<StringId> },
    /// A deprecated API marker.
    Deprecated { message: Option<StringId> },
    /// A no-managed allocation request.
    NoManaged,
    /// A no-heap allocation request.
    NoHeap,
    /// A must-use result marker.
    MustUse,
    /// A tainted data source marker.
    Taint { label: Option<StringId> },
    /// A tainted data sink marker.
    Sink { label: Option<StringId> },
    /// A taint sanitizer marker.
    Sanitizer { label: Option<StringId> },
    /// A user-defined marker.
    Tag { label: Option<StringId> },
}

/// Sparse semantic attributes attached to a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SymbolAttributes(pub Vec<SymbolAttribute>);

impl SymbolAttributes {
    /// Return the binding name override, if the symbol is a binding.
    pub fn binding_name(&self) -> Option<Option<StringId>> {
        self.0.iter().find_map(|attribute| match attribute {
            SymbolAttribute::Binding { name } => Some(*name),
            _ => None,
        })
    }

    /// Return the external name override, if the symbol is external.
    pub fn extern_name(&self) -> Option<Option<StringId>> {
        self.0.iter().find_map(|attribute| match attribute {
            SymbolAttribute::Extern { name } => Some(*name),
            _ => None,
        })
    }

    /// Return the deprecation message, if the symbol is deprecated.
    pub fn deprecated_message(&self) -> Option<Option<StringId>> {
        self.0.iter().find_map(|attribute| match attribute {
            SymbolAttribute::Deprecated { message } => Some(*message),
            _ => None,
        })
    }

    /// Return whether the symbol asks for no managed allocation.
    pub fn is_no_managed(&self) -> bool {
        self.has_flag(SymbolAttributeFlag::NoManaged)
    }

    /// Return whether the symbol asks for no heap allocation.
    pub fn is_no_heap(&self) -> bool {
        self.has_flag(SymbolAttributeFlag::NoHeap)
    }

    /// Return whether the symbol result must be used.
    pub fn is_must_use(&self) -> bool {
        self.has_flag(SymbolAttributeFlag::MustUse)
    }

    /// Return taint source labels.
    pub fn taints(&self) -> impl Iterator<Item = Option<StringId>> + '_ {
        self.0.iter().filter_map(|attribute| match attribute {
            SymbolAttribute::Taint { label } => Some(*label),
            _ => None,
        })
    }

    /// Return taint sink labels.
    pub fn sinks(&self) -> impl Iterator<Item = Option<StringId>> + '_ {
        self.0.iter().filter_map(|attribute| match attribute {
            SymbolAttribute::Sink { label } => Some(*label),
            _ => None,
        })
    }

    /// Return sanitizer labels.
    pub fn sanitizers(&self) -> impl Iterator<Item = Option<StringId>> + '_ {
        self.0.iter().filter_map(|attribute| match attribute {
            SymbolAttribute::Sanitizer { label } => Some(*label),
            _ => None,
        })
    }

    /// Return whether a flag attribute exists.
    fn has_flag(&self, flag: SymbolAttributeFlag) -> bool {
        self.0.iter().any(|attribute| {
            matches!(
                (attribute, flag),
                (SymbolAttribute::NoManaged, SymbolAttributeFlag::NoManaged)
                    | (SymbolAttribute::NoHeap, SymbolAttributeFlag::NoHeap)
                    | (SymbolAttribute::MustUse, SymbolAttributeFlag::MustUse)
            )
        })
    }
}

/// A flag-like symbol attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolAttributeFlag {
    /// No managed allocation.
    NoManaged,
    /// No heap allocation.
    NoHeap,
    /// Must-use result.
    MustUse,
}
