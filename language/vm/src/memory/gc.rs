/// Summary statistics for a garbage collection cycle.
#[derive(Clone, Copy, Debug, Default)]
pub struct GcStats {
    /// Number of cells freed by the collection.
    pub freed_cells: usize,
    /// Number of live cells after the collection.
    pub live_cells: usize,
}
