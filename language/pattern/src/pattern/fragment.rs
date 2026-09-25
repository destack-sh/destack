use tspp_dir as dir;
use tspp_source::Span;

use crate::MetavariableUses;

/// The index of a structural fragment in a pattern.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FragmentId(pub u32);

/// Parsed DIR matched from a selected root.
#[derive(Debug)]
pub struct Fragment {
    /// The parsed nodes.
    pub(crate) tree: dir::Tree,
    /// The node compared with candidate nodes.
    pub(crate) root: dir::LocalNodeIdAny,
    /// The authored source occupied by the fragment.
    pub(crate) span: Span,
    /// The metavariable markers in the parsed nodes.
    pub(crate) uses: MetavariableUses,
}

impl Fragment {
    /// Return the parsed nodes.
    pub fn tree(&self) -> &dir::Tree {
        &self.tree
    }

    /// Return the node compared with candidate nodes.
    pub fn root(&self) -> dir::LocalNodeIdAny {
        self.root
    }

    /// Return the authored source occupied by the fragment.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Return the type required of candidate nodes.
    pub fn node_type(&self) -> dir::NodeType {
        self.root.ty
    }

    /// Return the metavariable markers in the parsed nodes.
    pub fn uses(&self) -> &MetavariableUses {
        &self.uses
    }
}
