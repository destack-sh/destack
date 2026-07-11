use destack_dir as dir;
use smallvec::SmallVec;

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
    /// Return the work origin anchored at this site.
    pub(in crate::check) fn origin(self) -> Origin {
        Origin::Node(self.node, self.scope)
    }
}

/// One stable value path root tracked by flow analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum FlowRoot {
    /// A source binding symbol.
    Symbol(dir::GlobalSymbolId),
    /// The contextual receiver.
    Receiver(dir::ReceiverKind),
}

/// One stable value path tracked by flow analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct FlowPath {
    /// The root value.
    pub(in crate::check) root: FlowRoot,
    /// The members or keys below the root.
    pub(in crate::check) segments: SmallVec<[dir::StaticKey; 2]>,
}

impl FlowPath {
    /// Create a path rooted at one binding symbol.
    pub(in crate::check) fn symbol(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            root: FlowRoot::Symbol(symbol),
            segments: SmallVec::new(),
        }
    }

    /// Create a path rooted at one receiver.
    pub(in crate::check) fn receiver(receiver: dir::ReceiverKind) -> Self {
        Self {
            root: FlowRoot::Receiver(receiver),
            segments: SmallVec::new(),
        }
    }

    /// Add one member or key.
    pub(in crate::check) fn push_segment(&mut self, key: dir::StaticKey) {
        self.segments.push(key);
    }

    /// Return whether this path is under another path.
    pub(in crate::check) fn starts_with(&self, prefix: &FlowPath) -> bool {
        // compare root before member path
        self.root == prefix.root && self.segments.starts_with(&prefix.segments)
    }

    /// Return this path split into its parent and final segment.
    pub(in crate::check) fn split_last(&self) -> Option<(Self, dir::StaticKey)> {
        let key = self.segments.last().copied()?;
        let mut parent = self.clone();

        parent.segments.pop();

        Some((parent, key))
    }
}
