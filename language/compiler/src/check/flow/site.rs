use destack_dir as dir;

use crate::check::{FlowPointId, Origin};

/// One source use under a flow point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct FlowSite {
    /// The source node.
    pub(in crate::check) node: dir::GlobalNodeIdAny,
    /// The flow point where the node is used.
    pub(in crate::check) flow: FlowPointId,
    /// The generic template whose predicates the node assumes.
    pub(in crate::check) scope: Option<dir::GlobalGenericTemplateId>,
}

impl FlowSite {
    /// Return the check origin anchored at this site.
    pub(in crate::check) fn origin(self) -> Origin {
        Origin::Node(self.node, self.scope)
    }
}
