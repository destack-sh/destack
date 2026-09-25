use tspp_repository::ProviderContext;

/// Counted work metrics for one import attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::import) struct ImportStats {
    /// The number of active expression roots scanned.
    pub(in crate::import) roots: usize,
    /// The number of expressions inspected for module edges.
    pub(in crate::import) expressions: usize,
    /// The number of import declarations inspected.
    pub(in crate::import) import_clauses: usize,
    /// The number of re-export declarations inspected.
    pub(in crate::import) reexport_clauses: usize,
    /// The number of static guard conditions evaluated.
    pub(in crate::import) guards: usize,
    /// The number of module clauses skipped by static guards.
    pub(in crate::import) skipped: usize,
    /// The number of module specifiers resolved.
    pub(in crate::import) specifiers: usize,
    /// The number of package export selectors checked.
    pub(in crate::import) package_exports: usize,
    /// The number of candidate paths generated.
    pub(in crate::import) candidates: usize,
    /// The number of candidate paths probed for modules.
    pub(in crate::import) probes: usize,
}

impl ImportStats {
    /// Record these stats in one provider attempt.
    pub(in crate::import) fn record(self, context: &dyn ProviderContext) {
        context.record_counters(&[
            ("import.roots", self.roots as u64),
            ("import.expressions", self.expressions as u64),
            ("import.import_clauses", self.import_clauses as u64),
            ("import.reexport_clauses", self.reexport_clauses as u64),
            ("import.guards", self.guards as u64),
            ("import.skipped", self.skipped as u64),
            ("import.specifiers", self.specifiers as u64),
            ("import.package_exports", self.package_exports as u64),
            ("import.candidates", self.candidates as u64),
            ("import.probes", self.probes as u64),
        ]);
    }
}
