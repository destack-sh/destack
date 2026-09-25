use tspp_core::StringPool;
use tspp_dir as dir;

/// Protocol method selected by a subscript expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum SubscriptProtocol {
    /// Read through `a[i]`.
    Index,
    /// Write through `a[i] = value`.
    IndexSet,
}

impl SubscriptProtocol {
    /// Return the source member key for this protocol method.
    pub(in crate::sema) fn key(self, strings: &StringPool) -> dir::StaticKey {
        dir::StaticKey::Name(strings.intern(self.name()))
    }

    /// Return the language item for this protocol.
    pub(in crate::sema) fn item(self) -> dir::LanguageItem {
        match self {
            Self::Index => dir::LanguageItem::Index,
            Self::IndexSet => dir::LanguageItem::IndexSet,
        }
    }

    /// Return the source member name for this protocol method.
    fn name(self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::IndexSet => "indexSet",
        }
    }
}
