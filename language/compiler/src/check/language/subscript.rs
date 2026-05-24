use destack_core::StringPool;
use destack_dir as dir;

/// Protocol method selected by subscript syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SubscriptMethod {
    /// Read through `a[i]`.
    Index,
    /// Write through `a[i] = value`.
    IndexSet,
}

impl SubscriptMethod {
    /// Return the source member key for this protocol method.
    pub(in crate::check) fn key(self, strings: &StringPool) -> dir::StaticKey {
        dir::StaticKey::Name(strings.intern(self.name()))
    }

    /// Return the source member name for this protocol method.
    fn name(self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::IndexSet => "indexSet",
        }
    }
}
