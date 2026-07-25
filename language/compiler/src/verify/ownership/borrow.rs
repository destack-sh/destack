use destack_mir as mir;

/// Source that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum BorrowSource {
    /// Global or static storage.
    Static,
    /// Explicit lifetime slot.
    Lifetime(mir::LifetimeSlot),
    /// Owned storage.
    Owned,
    /// Managed storage.
    Managed {
        /// The storage space.
        space: mir::Space,
        /// The explicit lifetime keeping this managed handle alive.
        lifetime: Option<mir::Lifetime>,
    },
}

impl BorrowSource {
    /// Return whether this source can be borrowed in safe code.
    pub(super) fn allows_borrow(&self, access: mir::Access) -> bool {
        match self {
            Self::Managed {
                space: mir::Space::Shared,
                ..
            } => !access.is_exclusive(),
            Self::Static | Self::Lifetime(_) | Self::Owned | Self::Managed { .. } => true,
        }
    }

    /// Return whether this source lives at least as long as `other`.
    pub(super) fn outlives_source(&self, other: &BorrowSource) -> bool {
        match (self, other) {
            // static storage outlives every source
            (Self::Static, _) => true,
            // slots compare by identity without caller rows
            (Self::Lifetime(left), Self::Lifetime(right)) => left == right,
            // caller slots outlive the caller's own frame and region
            (Self::Lifetime(_), Self::Owned | Self::Managed { .. }) => true,
            // one frame outlives itself and its synchronous region
            (Self::Owned, Self::Owned) => true,
            (Self::Owned, Self::Managed { lifetime: None, .. }) => true,
            (Self::Managed { .. }, Self::Managed { .. }) => self == other,
            _ => false,
        }
    }

    /// Return whether this source is local to the current function.
    pub(super) fn is_function_local(&self) -> bool {
        match self {
            Self::Owned => true,
            Self::Managed { lifetime, .. } => lifetime.is_none(),
            Self::Static | Self::Lifetime(_) => false,
        }
    }

    /// Return whether this source is covered by a required lifetime.
    pub(super) fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        match self {
            // static storage outlives any requirement
            Self::Static => true,
            Self::Lifetime(slot) => required.includes_slot(slot.0),
            Self::Managed {
                lifetime: Some(lifetime),
                ..
            } => lifetime
                .terms
                .iter()
                .all(|term| required.terms.contains(term)),
            Self::Owned | Self::Managed { .. } => false,
        }
    }
}

/// Sources that keep a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BorrowSources {
    /// The borrow sources.
    sources: Vec<BorrowSource>,
}

/// Borrow sources keyed by SSA value path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BorrowMap {
    /// Stored source bindings.
    pub(super) bindings: Vec<BorrowBinding>,
}

/// Borrow sources for one value path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BorrowBinding {
    /// SSA value carrying the borrowed path.
    pub(super) value: mir::Value,
    /// Path inside the value.
    pub(super) path: mir::Path,
    /// Sources that keep the path live.
    pub(super) sources: BorrowSources,
}

impl BorrowSources {
    /// Create an empty source set.
    pub(super) fn none() -> Self {
        Self::default()
    }

    /// Create a source set from one source.
    pub(super) fn one(source: BorrowSource) -> Self {
        Self {
            sources: vec![source],
        }
    }

    /// Create a source set from many sources.
    pub(super) fn new(sources: impl IntoIterator<Item = BorrowSource>) -> Self {
        let mut unique = Vec::new();

        // retain insertion order while removing duplicates
        for source in sources {
            if !unique.contains(&source) {
                unique.push(source);
            }
        }

        Self { sources: unique }
    }

    /// Return whether the source set is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Return whether every source allows safe borrow creation.
    pub(super) fn allows_borrow(&self, access: mir::Access) -> bool {
        self.sources
            .iter()
            .all(|source| source.allows_borrow(access))
    }

    /// Return whether every source may escape the function.
    pub(super) fn is_escaping(&self) -> bool {
        !self.is_empty()
            && self
                .sources
                .iter()
                .all(|source| !source.is_function_local())
    }

    /// Return whether any source is local to this function.
    pub(super) fn has_function_local_source(&self) -> bool {
        self.sources.iter().any(BorrowSource::is_function_local)
    }

    /// Return whether these sources satisfy a required MIR lifetime.
    pub(super) fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        self.sources
            .iter()
            .all(|source| source.is_covered_by(required))
    }

    /// Return whether these sources live at least as long as `other`.
    ///
    /// Both durations are the minimum over their sources, so every source
    /// here must outlive at least one source on the other side.
    pub(super) fn outlives(&self, other: &BorrowSources) -> bool {
        self.sources.iter().all(|source| {
            other
                .sources
                .iter()
                .any(|shorter| source.outlives_source(shorter))
        })
    }

    /// Merge two source sets.
    pub(super) fn merge(&self, other: &Self) -> Self {
        Self::new(self.sources.iter().chain(&other.sources).cloned())
    }
}

impl BorrowMap {
    /// Bind one value to borrow sources.
    pub(super) fn insert(&mut self, value: mir::Value, sources: BorrowSources) {
        self.insert_at(value, mir::Path::root(), sources);
    }

    /// Bind one value path to borrow sources.
    pub(super) fn insert_at(&mut self, value: mir::Value, path: mir::Path, sources: BorrowSources) {
        if sources.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate rows
        if let Some(current) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.value == value && binding.path == path)
        {
            current.sources = sources;
            return;
        }

        self.bindings.push(BorrowBinding {
            value,
            path,
            sources,
        });
    }

    /// Bind all source paths for one value.
    pub(super) fn insert_bindings(
        &mut self,
        value: mir::Value,
        bindings: Vec<(mir::Path, BorrowSources)>,
    ) {
        let mut root_sources = BorrowSources::none();

        // keep a conservative root view for whole-value checks
        for (path, sources) in bindings {
            root_sources = root_sources.merge(&sources);
            self.insert_at(value, path, sources);
        }

        self.insert(value, root_sources);
    }

    /// Return borrow sources for one value path.
    pub(super) fn get_at(&self, value: mir::Value, path: &mir::Path) -> Option<&BorrowSources> {
        self.bindings.iter().find_map(|binding| {
            (binding.value == value && &binding.path == path).then_some(&binding.sources)
        })
    }

    /// Merge one value path source set.
    pub(super) fn merge_sources_at(
        &mut self,
        value: mir::Value,
        path: &mir::Path,
        sources: &BorrowSources,
    ) {
        // accumulate sources from repeated paths
        let sources = self
            .get_at(value, path)
            .map(|current| current.merge(sources))
            .unwrap_or_else(|| sources.clone());

        self.insert_at(value, path.clone(), sources);
    }

    /// Bind successor parameter sources.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        let mut bindings = Vec::new();

        // copy argument path facts to the successor parameter
        for binding in &self.bindings {
            if binding.value == argument {
                bindings.push((binding.path.clone(), binding.sources.clone()));
            }
        }

        for (path, sources) in bindings {
            self.insert_at(parameter, path, sources);
        }

        // update value references embedded in dynamic paths
        for binding in &mut self.bindings {
            binding.path.replace_value(argument, parameter);
        }
    }
}
