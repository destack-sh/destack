use tspp_core::{SectionBuilder, SectionImage, SectionStorage};
use tspp_heap::{TraceTable, TraceView};

/// Section-backed trace table used by heap benchmarks.
pub(crate) struct BenchTraceTable {
    /// Packed section storage.
    storage: SectionStorage,
    /// Packed heap trace table.
    traces: TraceTable,
    /// Number of top-level traces.
    trace_count: usize,
}

impl BenchTraceTable {
    /// Build one empty benchmark trace table.
    pub(crate) fn new() -> Self {
        Self::from_mir(&tspp_mir::TraceTable::new())
    }

    /// Build one benchmark trace table from MIR traces.
    pub(crate) fn from_mir(source: &tspp_mir::TraceTable) -> Self {
        let mut sections = SectionBuilder::new();
        let trace_count = source.traces().len();
        let traces = TraceTable::pack(&mut sections, source);
        let storage = sections.build();

        Self {
            storage,
            traces,
            trace_count,
        }
    }

    /// Return one borrowed trace view.
    pub(crate) fn view(&self) -> TraceView<'_> {
        // SAFETY: test storage is built exclusively through SectionBuilder.
        let sections = unsafe { SectionImage::new(&self.storage) };

        self.traces.view(sections)
    }

    /// Return the top-level trace count.
    pub(crate) fn trace_count(&self) -> usize {
        self.trace_count
    }
}
