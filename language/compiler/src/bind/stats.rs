use tspp_repository::ProviderContext;

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
    /// Record these stats in one provider attempt.
    pub(in crate::bind) fn record(self, context: &dyn ProviderContext) {
        context.record_counters(&[
            ("bind.files", self.files as u64),
            ("bind.roots", self.roots as u64),
            ("bind.expressions", self.expressions as u64),
            ("bind.declarations", self.declarations as u64),
            ("bind.patterns", self.patterns as u64),
            ("bind.type_expressions", self.type_expressions as u64),
        ]);
    }
}
