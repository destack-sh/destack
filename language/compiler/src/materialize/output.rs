use {destack_dir as dir, destack_mir as mir};

/// Output produced by materializing a comptime slot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComptimeOutput {
    /// Static expression produced for DIR patching.
    pub dir: Option<dir::StaticTerm>,
    /// MIR constant derived from the comptime result.
    pub mir: Option<mir::Constant>,
}
