use tspp_repository::ProviderContext;

/// Counted work metrics for one export attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::export) struct ExportStats {
    /// The number of active expression roots scanned.
    pub(in crate::export) roots: usize,
    /// The number of expressions inspected for static visibility.
    pub(in crate::export) visibility_expressions: usize,
    /// The number of expressions inspected for explicit export clauses.
    pub(in crate::export) export_expressions: usize,
    /// The number of declaration symbols scanned.
    pub(in crate::export) scanned_symbols: usize,
    /// The number of static guard conditions evaluated.
    pub(in crate::export) guards: usize,
    /// The number of nodes skipped by static guards.
    pub(in crate::export) skipped: usize,
    /// The number of static visibility checks requested.
    pub(in crate::export) static_checks: usize,
    /// The number of static visibility cache hits.
    pub(in crate::export) static_cache_hits: usize,
}

impl ExportStats {
    /// Record these stats in one provider attempt.
    pub(in crate::export) fn record(self, context: &dyn ProviderContext) {
        context.record_counters(&[
            ("export.roots", self.roots as u64),
            (
                "export.visibility_expressions",
                self.visibility_expressions as u64,
            ),
            ("export.export_expressions", self.export_expressions as u64),
            ("export.scanned_symbols", self.scanned_symbols as u64),
            ("export.guards", self.guards as u64),
            ("export.skipped", self.skipped as u64),
            ("export.static_checks", self.static_checks as u64),
            ("export.static_cache_hits", self.static_cache_hits as u64),
        ]);
    }
}
