use destack_mir as mir;

/// Source that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum BorrowSource {
    /// Global or static storage.
    Static,
    /// Function parameter by index.
    Parameter(u32),
    /// Owned storage.
    Owned,
    /// Managed storage.
    Managed {
        /// The storage space.
        space: mir::Space,
        /// The parameter keeping this managed handle alive.
        parameter: Option<u32>,
    },
}

/// Suspension rule for one borrow source set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BorrowSuspension {
    /// The sources are stable across suspension.
    Stable,
    /// The sources require caller proof before crossing suspension.
    Requires(Vec<mir::Lifetime>),
    /// At least one source cannot cross suspension.
    Rejected,
}

impl BorrowSource {
    /// Return whether this source can be borrowed in safe code.
    pub(super) fn allows_borrow(&self, access: mir::Access) -> bool {
        match self {
            Self::Managed {
                space: mir::Space::Shared,
                ..
            } => !access.is_exclusive(),
            Self::Static | Self::Parameter(_) | Self::Owned | Self::Managed { .. } => true,
        }
    }

    /// Return the suspension rule for this source.
    fn suspension(&self) -> BorrowSuspension {
        match self {
            Self::Static | Self::Owned => BorrowSuspension::Stable,
            Self::Parameter(index) => {
                BorrowSuspension::Requires(vec![mir::Lifetime::parameter(*index)])
            }
            Self::Managed { .. } => BorrowSuspension::Rejected,
        }
    }

    /// Return whether this source is local to the current function.
    pub(super) fn is_function_local(&self) -> bool {
        match self {
            Self::Owned => true,
            Self::Managed { parameter, .. } => parameter.is_none(),
            Self::Static | Self::Parameter(_) => false,
        }
    }

    /// Return whether this source is covered by a required lifetime.
    pub(super) fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        match self {
            Self::Static => required.includes_static(),
            Self::Parameter(index) => required.includes_parameter(*index),
            Self::Managed {
                parameter: Some(index),
                ..
            } => required.includes_parameter(*index),
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

/// Borrow sources keyed by SSA value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct BorrowMap {
    /// Stored source bindings.
    pub(super) sources: Vec<(mir::Value, BorrowSources)>,
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

    /// Iterate over stored sources.
    pub(super) fn iter(&self) -> impl Iterator<Item = &BorrowSource> {
        self.sources.iter()
    }

    /// Return whether every source allows safe borrow creation.
    pub(super) fn allows_borrow(&self, access: mir::Access) -> bool {
        self.sources
            .iter()
            .all(|source| source.allows_borrow(access))
    }

    /// Return the suspension rule for all sources.
    pub(super) fn suspension(&self) -> BorrowSuspension {
        let mut lifetimes = Vec::new();

        // combine source predicates
        for source in &self.sources {
            match source.suspension() {
                BorrowSuspension::Stable => {}
                BorrowSuspension::Requires(required) => {
                    for lifetime in required {
                        if !lifetimes.contains(&lifetime) {
                            lifetimes.push(lifetime);
                        }
                    }
                }
                BorrowSuspension::Rejected => return BorrowSuspension::Rejected,
            }
        }

        if lifetimes.is_empty() {
            BorrowSuspension::Stable
        } else {
            BorrowSuspension::Requires(lifetimes)
        }
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

    /// Return the external lifetime covered by these sources.
    pub(super) fn lifetime(&self) -> mir::Lifetime {
        mir::Lifetime::new(self.sources.iter().filter_map(|source| match source {
            BorrowSource::Static => Some(mir::LifetimeOrigin::Static),
            BorrowSource::Parameter(index)
            | BorrowSource::Managed {
                parameter: Some(index),
                ..
            } => Some(mir::LifetimeOrigin::Parameter(*index)),
            BorrowSource::Owned
            | BorrowSource::Managed {
                parameter: None, ..
            } => None,
        }))
    }

    /// Merge two source sets.
    pub(super) fn merge(&self, other: &Self) -> Self {
        Self::new(self.sources.iter().chain(&other.sources).cloned())
    }
}

impl BorrowMap {
    /// Bind one value to borrow sources.
    pub(super) fn insert(&mut self, value: mir::Value, sources: BorrowSources) {
        if sources.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate rows
        if let Some((_, current)) = self.sources.iter_mut().find(|(slot, _)| *slot == value) {
            *current = sources;
            return;
        }

        self.sources.push((value, sources));
    }

    /// Return borrow sources for one value.
    pub(super) fn get(&self, value: mir::Value) -> Option<&BorrowSources> {
        self.sources
            .iter()
            .find_map(|(slot, sources)| (*slot == value).then_some(sources))
    }

    /// Merge one value source set.
    pub(super) fn merge_sources(&mut self, value: mir::Value, sources: &BorrowSources) {
        // accumulate sources from repeated paths
        let sources = self
            .get(value)
            .map(|current| current.merge(sources))
            .unwrap_or_else(|| sources.clone());

        self.insert(value, sources);
    }

    /// Bind successor parameter sources.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        if let Some(sources) = self.get(argument).cloned() {
            self.insert(parameter, sources);
        }
    }
}
