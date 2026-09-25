use tspp_repository::ProviderContext;

/// Counted work metrics for one resolve attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::resolve) struct ResolveStats {
    /// The number of active expression roots walked.
    pub(in crate::resolve) roots: usize,
    /// The number of expression nodes visited.
    pub(in crate::resolve) expressions: usize,
    /// The number of type expression nodes visited.
    pub(in crate::resolve) type_expressions: usize,
    /// The number of import clauses collected.
    pub(in crate::resolve) import_clauses: usize,
    /// The number of re-export clauses collected.
    pub(in crate::resolve) reexport_clauses: usize,
    /// The number of required global keys.
    pub(in crate::resolve) required_globals: usize,
    /// The number of language items recorded from active roots.
    pub(in crate::resolve) language_item_uses: usize,
    /// The number of export lookup cache misses.
    pub(in crate::resolve) export_cache_misses: usize,
    /// The number of export lookup cache hits.
    pub(in crate::resolve) export_cache_hits: usize,
    /// The number of export lookup cycle hits.
    pub(in crate::resolve) export_cycle_hits: usize,
    /// The number of local binding lookups.
    pub(in crate::resolve) local_binding_lookups: usize,
    /// The number of import items resolved.
    pub(in crate::resolve) import_items: usize,
    /// The number of re-export items resolved.
    pub(in crate::resolve) reexport_items: usize,
    /// The number of export tables loaded.
    pub(in crate::resolve) export_table_loads: usize,
}

impl ResolveStats {
    /// Fold observed export lookup counters into this record.
    pub(in crate::resolve) fn record_exports(&mut self, exports: crate::export::ExportLookupStats) {
        self.export_cache_hits = exports.cache_hits;
        self.export_cache_misses = exports.cache_misses;
        self.export_cycle_hits = exports.cycle_hits;
        self.export_table_loads = exports.table_loads;
    }

    /// Record these stats in one provider attempt.
    pub(in crate::resolve) fn record(self, context: &dyn ProviderContext) {
        context.record_counters(&[
            ("resolve.roots", self.roots as u64),
            ("resolve.expressions", self.expressions as u64),
            ("resolve.type_expressions", self.type_expressions as u64),
            ("resolve.import_clauses", self.import_clauses as u64),
            ("resolve.reexport_clauses", self.reexport_clauses as u64),
            ("resolve.required_globals", self.required_globals as u64),
            ("resolve.language_item_uses", self.language_item_uses as u64),
            (
                "resolve.export_cache_misses",
                self.export_cache_misses as u64,
            ),
            ("resolve.export_cache_hits", self.export_cache_hits as u64),
            ("resolve.export_cycle_hits", self.export_cycle_hits as u64),
            (
                "resolve.local_binding_lookups",
                self.local_binding_lookups as u64,
            ),
            ("resolve.import_items", self.import_items as u64),
            ("resolve.reexport_items", self.reexport_items as u64),
            ("resolve.export_table_loads", self.export_table_loads as u64),
        ]);
    }
}
