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
    /// The number of profile global modules read.
    pub(in crate::resolve) global_modules: usize,
    /// The number of syntax-required language items.
    pub(in crate::resolve) required_language_items: usize,
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
    /// Render these stats as stable metadata lines.
    pub(in crate::resolve) fn render_metadata(self) -> String {
        let mut lines = vec![
            format!("resolve.stats.roots={}", self.roots),
            format!("resolve.stats.expressions={}", self.expressions),
            format!("resolve.stats.types={}", self.type_expressions),
        ];

        // include only active work classes
        if self.import_clauses != 0 || self.reexport_clauses != 0 {
            lines.push(format!(
                "resolve.stats.clauses=import:{},reexport:{}",
                self.import_clauses, self.reexport_clauses
            ));
        }

        // include only required global work
        if self.required_globals != 0 || self.global_modules != 0 {
            lines.push(format!(
                "resolve.stats.globals=required:{},modules:{}",
                self.required_globals, self.global_modules
            ));
        }

        // include only syntax language item work
        if self.required_language_items != 0 {
            lines.push(format!(
                "resolve.stats.language=required:{}",
                self.required_language_items
            ));
        }

        // include only export lookup work
        if self.export_cache_misses != 0
            || self.export_cache_hits != 0
            || self.export_cycle_hits != 0
        {
            lines.push(format!(
                "resolve.stats.exports=miss:{},hit:{},cycle:{}",
                self.export_cache_misses, self.export_cache_hits, self.export_cycle_hits
            ));
        }

        // include lookup work when present
        if self.local_binding_lookups != 0 || self.import_items != 0 || self.reexport_items != 0 {
            lines.push(format!(
                "resolve.stats.lookups.local={}",
                self.local_binding_lookups
            ));
            lines.push(format!(
                "resolve.stats.lookups.import_items={}",
                self.import_items
            ));
            lines.push(format!(
                "resolve.stats.lookups.reexport_items={}",
                self.reexport_items
            ));
        }

        // include artifact load work when present
        if self.export_table_loads != 0 || self.global_modules != 0 {
            lines.push(format!(
                "resolve.stats.loads.exports={}",
                self.export_table_loads
            ));
            lines.push(format!(
                "resolve.stats.loads.globals={}",
                self.global_modules
            ));
        }

        lines.join("\n")
    }
}
