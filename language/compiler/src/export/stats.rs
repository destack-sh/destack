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
    /// Render these stats as stable metadata lines.
    pub(in crate::export) fn render_metadata(self) -> String {
        let mut lines = vec![
            format!("export.stats.roots={}", self.roots),
            format!(
                "export.stats.expressions=visibility:{},export:{}",
                self.visibility_expressions, self.export_expressions
            ),
            format!("export.stats.symbols=scanned:{}", self.scanned_symbols),
        ];

        // include only static guard work
        if self.guards != 0 || self.skipped != 0 {
            lines.push(format!(
                "export.stats.guards=evaluated:{},skipped:{}",
                self.guards, self.skipped
            ));
        }

        // include static visibility cache work when present
        if self.static_checks != 0 || self.static_cache_hits != 0 {
            lines.push(format!("export.stats.static.checks={}", self.static_checks));
            lines.push(format!(
                "export.stats.static.cache_hits={}",
                self.static_cache_hits
            ));
        }

        lines.join("\n")
    }
}
