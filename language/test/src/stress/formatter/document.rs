use destack_fir::format::DocumentStats;

/// Format FIR document statistics for terminal output.
pub(super) fn format_document_stats(stats: DocumentStats) -> String {
    format!(
        "top {}, recursive {}, stored {} instructions / {} slices / {} bytes, token {}, text {}, space {}, line {}, tag {}, slices {} / {} instructions, best fitting {} / {} variants / {} instructions",
        stats.top_level_instructions,
        stats.recursive_instructions,
        stats.stored_instructions,
        stats.stored_slices,
        stats.bytes(),
        stats.tokens,
        stats.texts,
        stats.spaces,
        stats.lines,
        stats.tags,
        stats.slices,
        stats.slice_instructions,
        stats.best_fitting,
        stats.best_fitting_variants,
        stats.best_fitting_instructions,
    )
}
