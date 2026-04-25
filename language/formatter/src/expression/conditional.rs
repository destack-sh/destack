/// The layout position of one conditional inside its parent chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalLayout {
    /// This conditional is not nested inside another conditional.
    Root,
    /// This conditional is the test side of a parent conditional.
    NestedTest,
    /// This conditional is the consequent side of a parent conditional.
    NestedConsequent,
    /// This conditional is the alternate side of a parent conditional.
    NestedAlternate,
}

impl ConditionalLayout {
    /// Return whether this conditional groups like a root conditional.
    pub(crate) fn groups_at_root(self) -> bool {
        matches!(self, Self::Root | Self::NestedTest)
    }

    /// Return whether this conditional is a nested test.
    pub(crate) fn is_nested_test(self) -> bool {
        matches!(self, Self::NestedTest)
    }

    /// Return whether this conditional is a nested alternate.
    pub(crate) fn is_nested_alternate(self) -> bool {
        matches!(self, Self::NestedAlternate)
    }
}
