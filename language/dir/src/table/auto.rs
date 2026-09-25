use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::LanguageItem;

/// Interface whose implementation can be provided by compiler rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AutoInterface {
    /// Values supported by atomic storage.
    AtomicSafe,
    /// Ordered comparison interface.
    Compare,
    /// Complete by-value storage representation.
    Concrete,
    /// Implicit duplication without ownership transfer.
    Copy,
    /// Explicit clone interface.
    Clone,
    /// Debug formatting interface.
    Debug,
    /// User-facing display formatting interface.
    Display,
    /// Conventional default-value interface.
    Default,
    /// Structured deserialization interface.
    Deserialize,
    /// Runtime erasure marker for `Dynamic<T>`.
    DynamicSafe,
    /// Owned values that run a hook when their storage ends.
    Drop,
    /// Equality interface.
    Equal,
    /// Builtin float marker for any float format.
    Float,
    /// Float literals and builtin float formats.
    FloatDomain,
    /// Hashing interface.
    Hash,
    /// Builtin integer marker for any integer width.
    Integer,
    /// Integer literals, intervals, and builtin integer types.
    IntegerDomain,
    /// Partial ordered comparison interface.
    PartialCompare,
    /// Partial equality interface.
    PartialEqual,
    /// Structured serialization interface.
    Serialize,
    /// Shared-storage safety marker.
    SharedSafe,
    /// Builtin strict equality marker.
    StrictEqual,
    /// Pin-move capability.
    Unpin,
    /// Zero-byte initialization capability.
    Zeroable,
}

impl AutoInterface {
    /// Every auto interface in declaration order.
    pub const ALL: [Self; 24] = [
        Self::AtomicSafe,
        Self::Compare,
        Self::Concrete,
        Self::Copy,
        Self::Clone,
        Self::Debug,
        Self::Display,
        Self::Default,
        Self::Deserialize,
        Self::DynamicSafe,
        Self::Drop,
        Self::Equal,
        Self::Float,
        Self::FloatDomain,
        Self::Hash,
        Self::Integer,
        Self::IntegerDomain,
        Self::PartialCompare,
        Self::PartialEqual,
        Self::Serialize,
        Self::SharedSafe,
        Self::StrictEqual,
        Self::Unpin,
        Self::Zeroable,
    ];

    /// Return the auto interface named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::AtomicSafe => Some(Self::AtomicSafe),
            LanguageItem::Drop => Some(Self::Drop),
            LanguageItem::Compare => Some(Self::Compare),
            LanguageItem::Concrete => Some(Self::Concrete),
            LanguageItem::Copy => Some(Self::Copy),
            LanguageItem::Clone => Some(Self::Clone),
            LanguageItem::Debug => Some(Self::Debug),
            LanguageItem::Default => Some(Self::Default),
            LanguageItem::Deserialize => Some(Self::Deserialize),
            LanguageItem::Display => Some(Self::Display),
            LanguageItem::DynamicSafe => Some(Self::DynamicSafe),
            LanguageItem::Equal => Some(Self::Equal),
            LanguageItem::Float => Some(Self::Float),
            LanguageItem::FloatDomain => Some(Self::FloatDomain),
            LanguageItem::Hash => Some(Self::Hash),
            LanguageItem::Integer => Some(Self::Integer),
            LanguageItem::IntegerDomain => Some(Self::IntegerDomain),
            LanguageItem::PartialCompare => Some(Self::PartialCompare),
            LanguageItem::PartialEqual => Some(Self::PartialEqual),
            LanguageItem::Serialize => Some(Self::Serialize),
            LanguageItem::SharedSafe => Some(Self::SharedSafe),
            LanguageItem::StrictEqual => Some(Self::StrictEqual),
            LanguageItem::Unpin => Some(Self::Unpin),
            LanguageItem::Zeroable => Some(Self::Zeroable),
            _ => None,
        }
    }

    /// Return the source-facing interface name.
    pub fn name(self) -> &'static str {
        match self {
            Self::AtomicSafe => "AtomicSafe",
            Self::Compare => "Compare",
            Self::Concrete => "Concrete",
            Self::Copy => "Copy",
            Self::Clone => "Clone",
            Self::Debug => "Debug",
            Self::Default => "Default",
            Self::Deserialize => "Deserialize",
            Self::Display => "Display",
            Self::DynamicSafe => "DynamicSafe",
            Self::Drop => "Drop",
            Self::Equal => "Equal",
            Self::Float => "Float",
            Self::FloatDomain => "FloatDomain",
            Self::Hash => "Hash",
            Self::Integer => "Integer",
            Self::IntegerDomain => "IntegerDomain",
            Self::PartialCompare => "PartialCompare",
            Self::PartialEqual => "PartialEqual",
            Self::Serialize => "Serialize",
            Self::SharedSafe => "SharedSafe",
            Self::StrictEqual => "StrictEqual",
            Self::Unpin => "Unpin",
            Self::Zeroable => "Zeroable",
        }
    }

    /// Return whether this interface is memberless.
    pub fn is_marker(self) -> bool {
        match self {
            Self::AtomicSafe
            | Self::Concrete
            | Self::Copy
            | Self::DynamicSafe
            | Self::SharedSafe
            | Self::StrictEqual
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::IntegerDomain
            | Self::Float
            | Self::FloatDomain => true,
            Self::Compare
            | Self::Clone
            | Self::Debug
            | Self::Default
            | Self::Deserialize
            | Self::Display
            | Self::Drop
            | Self::Equal
            | Self::Hash
            | Self::PartialCompare
            | Self::PartialEqual
            | Self::Serialize => false,
        }
    }

    /// Return whether satisfying this interface can generate members.
    pub fn has_generated_members(self) -> bool {
        match self {
            Self::Compare
            | Self::Clone
            | Self::Debug
            | Self::Default
            | Self::Deserialize
            | Self::Display
            | Self::Equal
            | Self::Hash
            | Self::PartialCompare
            | Self::PartialEqual
            | Self::Serialize => true,
            Self::AtomicSafe
            | Self::Copy
            | Self::DynamicSafe
            | Self::SharedSafe
            | Self::StrictEqual
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::IntegerDomain
            | Self::Float
            | Self::FloatDomain
            | Self::Drop
            | Self::Concrete => false,
        }
    }

    /// Return whether an unsafe extension may assume this interface.
    pub fn permits_unsafe_implementation(self) -> bool {
        matches!(self, Self::SharedSafe | Self::Unpin | Self::Zeroable)
    }

    /// Return whether the compiler derives this interface field-wise without annotation.
    /// Return the member name a derivation of this interface implements.
    pub fn derived_member(self) -> Option<&'static str> {
        match self {
            Self::Clone => Some("clone"),
            Self::Debug => Some("debug"),
            Self::Display => Some("display"),
            Self::Equal | Self::PartialEqual => Some("equal"),
            Self::Hash => Some("hash"),
            Self::Default => Some("default"),
            _ => None,
        }
    }

    pub fn is_auto_derivable(self) -> bool {
        matches!(
            self,
            Self::Copy
                | Self::Clone
                | Self::Debug
                | Self::Display
                | Self::Equal
                | Self::PartialEqual
                | Self::Hash
                | Self::SharedSafe
        )
    }

    /// Return whether the compiler implements this interface without declarations.
    pub fn has_builtin_implementation(self) -> bool {
        // include scalar ordering, which the compiler decides outside the auto set
        self.is_marker()
            || self.is_auto_derivable()
            || matches!(self, Self::Compare | Self::PartialCompare | Self::Default)
    }

    /// Return whether this interface takes the compared value as its argument.
    pub fn has_receiver_argument(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::PartialEqual
                | Self::Compare
                | Self::PartialCompare
                | Self::StrictEqual
        )
    }

    /// Iterate every auto interface in declaration order.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter()
    }

    /// Return whether a class may derive this interface.
    pub fn derives_over_class(self) -> bool {
        !matches!(self, Self::Default | Self::Zeroable)
    }

    /// Return whether a written derive decorator may name this interface.
    pub fn is_derivable(self) -> bool {
        self.is_auto_derivable()
            || matches!(
                self,
                Self::Default
                    | Self::Compare
                    | Self::PartialCompare
                    | Self::Serialize
                    | Self::Deserialize
            )
    }
}

impl From<AutoInterface> for LanguageItem {
    fn from(interface: AutoInterface) -> Self {
        match interface {
            AutoInterface::AtomicSafe => Self::AtomicSafe,
            AutoInterface::Compare => Self::Compare,
            AutoInterface::Concrete => Self::Concrete,
            AutoInterface::Copy => Self::Copy,
            AutoInterface::Clone => Self::Clone,
            AutoInterface::Debug => Self::Debug,
            AutoInterface::Default => Self::Default,
            AutoInterface::Deserialize => Self::Deserialize,
            AutoInterface::Display => Self::Display,
            AutoInterface::DynamicSafe => Self::DynamicSafe,
            AutoInterface::Drop => Self::Drop,
            AutoInterface::Equal => Self::Equal,
            AutoInterface::Float => Self::Float,
            AutoInterface::FloatDomain => Self::FloatDomain,
            AutoInterface::Hash => Self::Hash,
            AutoInterface::Integer => Self::Integer,
            AutoInterface::IntegerDomain => Self::IntegerDomain,
            AutoInterface::PartialCompare => Self::PartialCompare,
            AutoInterface::PartialEqual => Self::PartialEqual,
            AutoInterface::Serialize => Self::Serialize,
            AutoInterface::SharedSafe => Self::SharedSafe,
            AutoInterface::StrictEqual => Self::StrictEqual,
            AutoInterface::Unpin => Self::Unpin,
            AutoInterface::Zeroable => Self::Zeroable,
        }
    }
}
