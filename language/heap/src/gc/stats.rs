/// Summary statistics for a garbage collection cycle.
#[derive(Clone, Copy, Debug, Default)]
pub struct GcStats {
    /// Number of cells freed by the collection.
    pub freed_cells: usize,
    /// Number of live cells after the collection.
    pub live_cells: usize,
    /// Number of bytes freed by the collection.
    pub freed_bytes: u64,
    /// Number of live bytes after the collection.
    pub live_bytes: u64,
    /// Total heap bytes after the collection.
    pub heap_bytes: u64,
}
