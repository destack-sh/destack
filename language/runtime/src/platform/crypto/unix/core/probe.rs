use std::time::{SystemTime, UNIX_EPOCH};

/// Build one unique probe identifier for temporary backend probes.
pub(crate) fn next_store_write_probe_identifier() -> String {
    // include one host-random component to avoid collisions in one runtime
    let mut random = [0u8; 8];
    let random = if getrandom::fill(&mut random).is_ok() {
        u64::from_le_bytes(random)
    } else {
        0
    };
    let process_id = std::process::id();

    // include one wall-clock component to avoid collisions across process restarts
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{process_id}.{timestamp}.{random}")
}
