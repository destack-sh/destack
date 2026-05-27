use std::mem;

use destack_heap::{
    DEFAULT_PAGE_BYTES, DEFAULT_SHARED_SMALL_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    DEFAULT_SMALL_BYTES, DEFAULT_YOUNG_BYTES, SizeClass, SizeClassTable,
};

const WORD_BYTES: usize = mem::size_of::<usize>();
const CURRENT_YOUNG_RANGE_RECORD_BYTES: usize = mem::size_of::<usize>() * 2;
const TARGET_YOUNG_RANGE_RECORD_BYTES: usize = mem::size_of::<u32>() * 3;
const TARGET_TRACE_MAP_ID_BYTES: usize = 4;
const DIRTY_CARD_BYTES: usize = DEFAULT_PAGE_BYTES / 32;

/// One reported size class row.
#[derive(Debug)]
struct Row {
    /// The smallest request routed to this class.
    min_request_bytes: usize,
    /// The largest request routed to this class.
    max_request_bytes: usize,
    /// The rounded slot byte length.
    slot_bytes: usize,
    /// The span byte length used by this slot class.
    span_bytes: usize,
    /// The number of slots in the span.
    slot_count: usize,
    /// The payload bytes wasted by size-class rounding.
    rounding_bytes: usize,
    /// The current local young scan metadata bytes per full-nursery allocation.
    current_young_scan_bytes: f64,
    /// The target local young scan metadata bytes per allocation.
    target_young_scan_bytes: f64,
    /// The current local young no-scan run metadata bytes per allocation.
    current_young_noscan_bytes: f64,
    /// The target local young no-scan run metadata bytes per allocation.
    target_young_noscan_bytes: f64,
    /// The current local mature scan metadata bytes per allocation.
    current_local_scan_bytes: f64,
    /// The target local mature scan metadata bytes per allocation.
    target_local_scan_bytes: f64,
    /// The current shared scan metadata bytes per allocation.
    current_shared_scan_bytes: f64,
    /// The target shared scan metadata bytes per allocation.
    target_shared_scan_bytes: f64,
}

fn main() {
    let table = SizeClassTable::default();
    let rows = table
        .classes
        .iter()
        .enumerate()
        .map(|(class_index, class)| row_for_class(class_index, *class, &table.classes))
        .collect::<Vec<_>>();

    print_policy();
    print_young_space_summary();
    print_size_table(&rows);
    print_notes();
}

/// Print the heap sizing policy used by the report.
fn print_policy() {
    println!("heap metadata overhead report");
    println!();
    println!("defaults");
    println!("  page bytes:             {}", DEFAULT_PAGE_BYTES);
    println!("  local young bytes:      {}", DEFAULT_YOUNG_BYTES);
    println!("  local small span bytes: {}", DEFAULT_SMALL_BYTES);
    println!("  shared span bytes:      {}", DEFAULT_SHARED_SMALL_BYTES);
    println!(
        "  allocation alignment:   {}",
        DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES
    );
    println!();
}

/// Print the current eager young-space metadata summary.
fn print_young_space_summary() {
    let reference_capacity = DEFAULT_YOUNG_BYTES.div_ceil(WORD_BYTES);
    let run_bucket_count = SizeClassTable::default().classes.len() * 2;
    let page_count = DEFAULT_YOUNG_BYTES.div_ceil(DEFAULT_PAGE_BYTES);

    let local_reference_bytes = bitmap_bytes(reference_capacity);
    let shared_reference_bytes = bitmap_bytes(reference_capacity);
    let page_runs_bytes = page_count * mem::size_of::<Option<usize>>();
    let run_buckets_bytes = run_bucket_count * mem::size_of::<Option<usize>>();
    let total_bytes =
        page_runs_bytes + run_buckets_bytes + local_reference_bytes + shared_reference_bytes;
    let metadata_percent = total_bytes as f64 / DEFAULT_YOUNG_BYTES as f64 * 100.0;

    println!("current eager local young metadata");
    println!("  page run owners:        {}", page_runs_bytes);
    println!("  no-scan run buckets:    {}", run_buckets_bytes);
    println!("  local reference bits:   {}", local_reference_bytes);
    println!("  shared reference bits:  {}", shared_reference_bytes);
    println!("  total:                  {total_bytes} ({metadata_percent:.1}% of young)");
    println!();
}

/// Print the per-allocation overhead table.
fn print_size_table(rows: &[Row]) {
    println!("per allocation metadata at default size classes");
    println!(
        "{:>13} {:>7} {:>7} {:>5} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "request",
        "slot",
        "span",
        "slots",
        "round",
        "young",
        "young*",
        "run",
        "run*",
        "local",
        "local*",
        "shared",
        "shared*",
    );
    println!(
        "{:>13} {:>7} {:>7} {:>5} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "range",
        "bytes",
        "bytes",
        "count",
        "bytes",
        "scan",
        "target",
        "noscan",
        "target",
        "scan",
        "target",
        "scan",
        "target",
    );

    for row in rows {
        let request_range = format!("{}..{}", row.min_request_bytes, row.max_request_bytes);

        println!(
            "{:>13} {:>7} {:>7} {:>5} {:>7} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
            request_range,
            row.slot_bytes,
            row.span_bytes,
            row.slot_count,
            row.rounding_bytes,
            row.current_young_scan_bytes,
            row.target_young_scan_bytes,
            row.current_young_noscan_bytes,
            row.target_young_noscan_bytes,
            row.current_local_scan_bytes,
            row.target_local_scan_bytes,
            row.current_shared_scan_bytes,
            row.target_shared_scan_bytes,
        );
    }

    println!();
}

/// Print interpretation notes for the report columns.
fn print_notes() {
    println!("legend");
    println!("  young:  current local young range allocation, amortized over a full nursery");
    println!(
        "  young*: target local young range allocation, compact record plus reference map bits"
    );
    println!("  run:    current local young no-scan fixed run");
    println!("  run*:   target local young no-scan fixed run");
    println!("  local:  current local mature small span, scan allocation");
    println!("  local*: target local mature type-homogeneous span, scan allocation");
    println!("  shared: current shared small span, scan allocation");
    println!("  shared*: target shared type-homogeneous span, scan allocation");
    println!();
    println!("large allocations");
    println!("  current: one side record, one page run, one trace map, dirty-card bits for local");
    println!("  target: one side record plus trace map id and card bits, no per-payload header");
}

/// Build one overhead row from the default heap sizing model.
fn row_for_class(class_index: usize, class: SizeClass, classes: &[SizeClass]) -> Row {
    let min_request_bytes = if class_index == 0 {
        1
    } else {
        classes[class_index - 1].bytes + 1
    };
    let span_bytes = class.span_bytes(DEFAULT_PAGE_BYTES, DEFAULT_SMALL_BYTES);
    let slot_count = (span_bytes / class.bytes).max(1);
    let rounding_bytes = class.bytes - min_request_bytes;

    Row {
        min_request_bytes,
        max_request_bytes: class.bytes,
        slot_bytes: class.bytes,
        span_bytes,
        slot_count,
        rounding_bytes,
        current_young_scan_bytes: current_young_scan_bytes(class.bytes),
        target_young_scan_bytes: target_young_scan_bytes(class.bytes),
        current_young_noscan_bytes: current_young_noscan_bytes(),
        target_young_noscan_bytes: target_young_noscan_bytes(),
        current_local_scan_bytes: current_local_scan_bytes(class.bytes, span_bytes, slot_count),
        target_local_scan_bytes: target_local_scan_bytes(span_bytes, slot_count),
        current_shared_scan_bytes: current_shared_scan_bytes(class.bytes),
        target_shared_scan_bytes: target_shared_scan_bytes(slot_count),
    }
}

/// Return current local young scan metadata bytes for one allocation.
fn current_young_scan_bytes(slot_bytes: usize) -> f64 {
    CURRENT_YOUNG_RANGE_RECORD_BYTES as f64 + reference_map_bytes(slot_bytes) + bit_bytes(2)
}

/// Return target local young scan metadata bytes for one allocation.
fn target_young_scan_bytes(slot_bytes: usize) -> f64 {
    TARGET_YOUNG_RANGE_RECORD_BYTES as f64 + reference_map_bytes(slot_bytes)
}

/// Return current local young no-scan run metadata bytes for one allocation.
fn current_young_noscan_bytes() -> f64 {
    bit_bytes(2)
}

/// Return target local young no-scan run metadata bytes for one allocation.
fn target_young_noscan_bytes() -> f64 {
    bit_bytes(1)
}

/// Return current local mature scan metadata bytes for one allocation.
fn current_local_scan_bytes(slot_bytes: usize, span_bytes: usize, slot_count: usize) -> f64 {
    let slot_bits = 2 + 2 * slot_bytes.div_ceil(WORD_BYTES);
    let card_bytes = card_bytes_per_slot(span_bytes, slot_count);

    bit_bytes(slot_bits) + card_bytes
}

/// Return target local mature scan metadata bytes for one allocation.
fn target_local_scan_bytes(span_bytes: usize, slot_count: usize) -> f64 {
    let slot_bits = 2;
    let card_bytes = card_bytes_per_slot(span_bytes, slot_count);
    let trace_id_bytes = TARGET_TRACE_MAP_ID_BYTES as f64 / slot_count as f64;

    bit_bytes(slot_bits) + card_bytes + trace_id_bytes
}

/// Return current shared scan metadata bytes for one allocation.
fn current_shared_scan_bytes(slot_bytes: usize) -> f64 {
    let slot_bits = 5 + 2 * slot_bytes.div_ceil(WORD_BYTES);

    bit_bytes(slot_bits)
}

/// Return target shared scan metadata bytes for one allocation.
fn target_shared_scan_bytes(slot_count: usize) -> f64 {
    let slot_bits = 3;
    let trace_id_bytes = TARGET_TRACE_MAP_ID_BYTES as f64 / slot_count as f64;

    bit_bytes(slot_bits) + trace_id_bytes
}

/// Return reference-map side metadata bytes for one slot payload.
fn reference_map_bytes(slot_bytes: usize) -> f64 {
    bit_bytes(2 * slot_bytes.div_ceil(WORD_BYTES))
}

/// Return dirty-card bytes amortized over one slot.
fn card_bytes_per_slot(span_bytes: usize, slot_count: usize) -> f64 {
    let card_count = span_bytes.div_ceil(DIRTY_CARD_BYTES);
    let card_bytes = bitmap_bytes(card_count);

    card_bytes as f64 / slot_count as f64
}

/// Return bitmap storage bytes for one bit capacity.
fn bitmap_bytes(bit_capacity: usize) -> usize {
    let word_bits = u64::BITS as usize;

    bit_capacity.div_ceil(word_bits) * mem::size_of::<u64>()
}

/// Return byte count represented by a bit count.
fn bit_bytes(bits: usize) -> f64 {
    bits as f64 / u8::BITS as f64
}
