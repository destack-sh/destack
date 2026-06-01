/// Counted work metrics for one bind attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::bind) struct BindStats {
    /// The number of active files visited.
    pub(in crate::bind) files: usize,
    /// The number of active expression roots bound.
    pub(in crate::bind) roots: usize,
    /// The number of expression nodes visited.
    pub(in crate::bind) expressions: usize,
    /// The number of declaration nodes visited.
    pub(in crate::bind) declarations: usize,
    /// The number of pattern nodes visited.
    pub(in crate::bind) patterns: usize,
    /// The number of type expression nodes visited.
    pub(in crate::bind) type_expressions: usize,
}

impl BindStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::bind) fn render_metadata(self) -> String {
        let mut lines = vec![
            format!("bind.stats.files={}", self.files),
            format!("bind.stats.roots={}", self.roots),
        ];

        // include visit work when present
        if self.expressions != 0
            || self.declarations != 0
            || self.patterns != 0
            || self.type_expressions != 0
        {
            lines.push(format!(
                "bind.stats.visited.expressions={}",
                self.expressions
            ));
            lines.push(format!(
                "bind.stats.visited.declarations={}",
                self.declarations
            ));
            lines.push(format!("bind.stats.visited.patterns={}", self.patterns));
            lines.push(format!(
                "bind.stats.visited.types={}",
                self.type_expressions
            ));
        }

        lines.join("\n")
    }
}
