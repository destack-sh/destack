use crate::resolve::state::ResolveState;

/// Derived and counted work metrics for one resolve attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// The number of resolved dependencies.
    pub(in crate::resolve) dependencies: usize,
    /// The number of resolved import symbols.
    pub(in crate::resolve) symbols: usize,
    /// The number of required global keys.
    pub(in crate::resolve) required_globals: usize,
    /// The number of profile global modules read.
    pub(in crate::resolve) global_modules: usize,
    /// The number of resolved global keys.
    pub(in crate::resolve) globals: usize,
    /// The number of syntax-required language items.
    pub(in crate::resolve) required_language_items: usize,
    /// The number of resolved language items.
    pub(in crate::resolve) language_items: usize,
    /// The number of export lookup computations.
    pub(in crate::resolve) export_lookups: usize,
    /// The number of export lookup cache hits.
    pub(in crate::resolve) export_cache_hits: usize,
    /// The number of export lookup cycle hits.
    pub(in crate::resolve) export_cycle_hits: usize,
    /// The number of recoverable diagnostics.
    pub(in crate::resolve) diagnostics: usize,
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

        // include only materialized import rows
        if self.dependencies != 0 || self.symbols != 0 {
            lines.push(format!(
                "resolve.stats.imports=dep:{},symbol:{}",
                self.dependencies, self.symbols
            ));
        }

        // include only required global work
        if self.required_globals != 0 || self.global_modules != 0 || self.globals != 0 {
            lines.push(format!(
                "resolve.stats.globals=required:{},module:{},loaded:{}",
                self.required_globals, self.global_modules, self.globals
            ));
        }

        // include only syntax language item work
        if self.required_language_items != 0 || self.language_items != 0 {
            lines.push(format!(
                "resolve.stats.language=required:{},loaded:{}",
                self.required_language_items, self.language_items
            ));
        }

        // include only export lookup work
        if self.export_lookups != 0 || self.export_cache_hits != 0 || self.export_cycle_hits != 0 {
            lines.push(format!(
                "resolve.stats.exports=miss:{},hit:{},cycle:{}",
                self.export_lookups, self.export_cache_hits, self.export_cycle_hits
            ));
        }

        // include only recoverable failures
        if self.diagnostics != 0 {
            lines.push(format!("resolve.stats.errors={}", self.diagnostics));
        }

        lines.join("\n")
    }
}

impl ResolveState<'_> {
    /// Return resolve work and output stats.
    pub(in crate::resolve) fn stats(&self) -> ResolveStats {
        ResolveStats {
            roots: self.roots,
            expressions: self.expressions,
            type_expressions: self.type_expressions,
            import_clauses: self.import_clauses,
            reexport_clauses: self.reexport_clauses,
            dependencies: self.imports.dependencies.len(),
            symbols: self.imports.target_by_symbol.len(),
            required_globals: self.required_global_keys.len(),
            global_modules: self.global_modules,
            globals: self.imports.global_symbol_by_key.len(),
            required_language_items: self.syntax_language_items.len(),
            language_items: self.imports.language_symbol_by_item.len(),
            export_lookups: self.export_lookups.len(),
            export_cache_hits: self.export_cache_hits,
            export_cycle_hits: self.export_cycle_hits,
            diagnostics: self.diagnostics.len(),
        }
    }
}
