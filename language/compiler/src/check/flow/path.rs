use destack_dir as dir;
use smallvec::SmallVec;

/// One stable value path tracked by flow analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct FlowPath {
    /// The root binding symbol.
    pub(in crate::check) root: dir::GlobalSymbolId,
    /// The selected members or keys below the root.
    pub(in crate::check) segments: SmallVec<[dir::StaticKey; 2]>,
}

impl FlowPath {
    /// Create a path rooted at one binding symbol.
    pub(in crate::check) fn symbol(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            root: symbol,
            segments: SmallVec::new(),
        }
    }

    /// Add one selected member or key.
    pub(in crate::check) fn push_segment(&mut self, key: dir::StaticKey) {
        self.segments.push(key);
    }

    /// Return whether this path is under another path.
    pub(in crate::check) fn starts_with(&self, prefix: &FlowPath) -> bool {
        // compare root before member path
        self.root == prefix.root && self.segments.starts_with(&prefix.segments)
    }
}
