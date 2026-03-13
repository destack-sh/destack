use {destack_dir as dir, destack_mir as mir};

/// Output produced by executing a comptime slot.
#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeOutput {
    /// Static expression produced for DIR patching.
    pub dir: Option<dir::StaticExpression>,
    /// MIR constant derived from the comptime result.
    pub mir: Option<mir::Constant>,
}

impl ComptimeOutput {
    /// Create an empty comptime output with deliberately void data.
    pub fn empty() -> Self {
        Self {
            dir: None,
            mir: None,
        }
    }
}
