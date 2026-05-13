use destack_mir as mir;

/// Root that keeps a borrowed value alive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum BorrowRoot {
    /// Global or static storage.
    Static,
    /// Function parameter by index.
    Parameter(u32),
    /// Local place inside the current function.
    Local(mir::Place),
}

/// Roots that keep a borrowed value alive.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BorrowRoots {
    /// The borrow roots.
    roots: Vec<BorrowRoot>,
}

/// Borrow roots keyed by SSA value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BorrowMap {
    /// Stored root bindings.
    pub(super) roots: Vec<(mir::Value, BorrowRoots)>,
}

impl BorrowRoots {
    /// Create an empty root set.
    pub(super) fn none() -> Self {
        Self::default()
    }

    /// Create a root set from one root.
    pub(super) fn one(root: BorrowRoot) -> Self {
        Self { roots: vec![root] }
    }

    /// Create a root set from many roots.
    pub(super) fn new(roots: impl IntoIterator<Item = BorrowRoot>) -> Self {
        let mut unique = Vec::new();

        // retain insertion order while removing duplicates
        for root in roots {
            if !unique.contains(&root) {
                unique.push(root);
            }
        }

        Self { roots: unique }
    }

    /// Return whether the root set is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Return whether every root may escape the function.
    pub(super) fn is_escaping(&self) -> bool {
        !self.is_empty()
            && self
                .roots
                .iter()
                .all(|root| !matches!(root, BorrowRoot::Local(_)))
    }

    /// Return whether any root is local to this function.
    pub(super) fn has_local_root(&self) -> bool {
        self.roots
            .iter()
            .any(|root| matches!(root, BorrowRoot::Local(_)))
    }

    /// Return whether these roots satisfy a required MIR lifetime.
    pub(super) fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        self.roots.iter().all(|root| match root {
            BorrowRoot::Static => required.includes_static(),
            BorrowRoot::Parameter(index) => required.includes_parameter(*index),
            BorrowRoot::Local(_) => false,
        })
    }

    /// Merge two root sets.
    pub(super) fn merge(&self, other: &Self) -> Self {
        Self::new(self.roots.iter().chain(&other.roots).cloned())
    }
}

impl BorrowMap {
    /// Bind one value to borrow roots.
    pub(super) fn insert(&mut self, value: mir::Value, roots: BorrowRoots) {
        if roots.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate rows
        if let Some((_, current)) = self.roots.iter_mut().find(|(slot, _)| *slot == value) {
            *current = roots;
            return;
        }

        self.roots.push((value, roots));
    }

    /// Return borrow roots for one value.
    pub(super) fn get(&self, value: mir::Value) -> Option<&BorrowRoots> {
        self.roots
            .iter()
            .find_map(|(slot, roots)| (*slot == value).then_some(roots))
    }

    /// Merge one value root set.
    pub(super) fn merge_roots(&mut self, value: mir::Value, roots: &BorrowRoots) {
        // accumulate roots from repeated paths
        let roots = self
            .get(value)
            .map(|current| current.merge(roots))
            .unwrap_or_else(|| roots.clone());

        self.insert(value, roots);
    }

    /// Bind successor parameter roots.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        if let Some(roots) = self.get(argument).cloned() {
            self.insert(parameter, roots);
        }
    }
}
