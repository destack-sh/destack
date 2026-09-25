use tspp_dir as dir;

use crate::{Fragment, MetavariableUses};

/// DIR nodes interpreted as one structural pattern.
#[derive(Clone, Copy)]
pub(crate) struct PatternNodes<'pattern> {
    /// The parsed pattern nodes.
    pub(crate) tree: dir::View<'pattern>,
    /// The metavariable markers over those nodes.
    pub(crate) uses: &'pattern MetavariableUses,
}

impl<'pattern> PatternNodes<'pattern> {
    /// Borrow one compiled structural fragment.
    pub(crate) fn new(fragment: &'pattern Fragment) -> Self {
        Self {
            tree: dir::View::new(fragment.tree()),
            uses: fragment.uses(),
        }
    }

    /// Interpret candidate nodes as a pattern without metavariables.
    pub(crate) fn plain(tree: dir::View<'pattern>, uses: &'pattern MetavariableUses) -> Self {
        Self { tree, uses }
    }

    /// Return the interpreted DIR nodes.
    pub(crate) fn tree(self) -> dir::View<'pattern> {
        self.tree
    }

    /// Return the metavariable markers over the nodes.
    pub(crate) fn uses(self) -> &'pattern MetavariableUses {
        self.uses
    }
}
