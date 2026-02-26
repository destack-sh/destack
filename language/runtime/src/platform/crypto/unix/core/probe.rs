use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global sequence used to generate unique write probe identifiers.
static STORE_WRITE_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Build one unique probe identifier for temporary backend probes.
pub(crate) fn next_store_write_probe_identifier() -> String {
    // include one process-local sequence to avoid collisions in one runtime
    let sequence = STORE_WRITE_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();

    // include one wall-clock component to avoid collisions across process restarts
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{process_id}.{timestamp}.{sequence}")
}
