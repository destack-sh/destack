use tspp_dir as dir;

use crate::sema::{FlowPointId, Origin};

/// One source use under a flow point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct FlowSite {
    /// The source node.
    pub(in crate::sema) node: dir::GlobalNodeIdAny,
    /// The flow point where the node is used.
    pub(in crate::sema) flow: FlowPointId,
    /// The generic template whose predicates the node assumes.
    pub(in crate::sema) scope: Option<dir::GlobalGenericTemplateId>,
}

impl FlowSite {
    /// Return the check origin anchored at this site.
    pub(in crate::sema) fn origin(self) -> Origin {
        Origin::Node(self.node, self.scope)
    }
}
