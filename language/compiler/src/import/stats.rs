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
    /// Render these stats as stable metadata lines.
    pub(in crate::import) fn render_metadata(self) -> String {
        let mut lines = vec![
            format!("import.stats.roots={}", self.roots),
            format!("import.stats.expressions={}", self.expressions),
            format!(
                "import.stats.clauses=import:{},reexport:{}",
                self.import_clauses, self.reexport_clauses
            ),
        ];

        // include only static guard work
        if self.guards != 0 || self.skipped != 0 {
            lines.push(format!(
                "import.stats.guards=evaluated:{},skipped:{}",
                self.guards, self.skipped
            ));
        }

        // include resolver work when present
        if self.specifiers != 0 || self.package_exports != 0 || self.candidates != 0 {
            lines.push(format!(
                "import.stats.resolve.specifiers={}",
                self.specifiers
            ));
            lines.push(format!(
                "import.stats.resolve.package_exports={}",
                self.package_exports
            ));
            lines.push(format!(
                "import.stats.resolve.candidates={}",
                self.candidates
            ));
            lines.push(format!("import.stats.resolve.probes={}", self.probes));
        }

        lines.join("\n")
    }
}
