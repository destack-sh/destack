use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform::ipc::{PipePair, PipePairVm, SharedMemoryMapping, SharedMemoryMappingVm};
pub(super) use crate::tests::platform::assert_runtime_error_code;

use super::HarnessValue;

/// Monotonic suffix used for unique IPC object names.
static IPC_NAME_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Build one unique IPC object name for one test case.
pub(super) fn unique_ipc_name(prefix: &str) -> String {
    // keep names short for strict POSIX object-name limits on some unix hosts
    let prefix = prefix
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(6)
        .collect::<String>();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos() as u64;
    let sequence = IPC_NAME_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    format!("/{prefix}{:x}{:x}", now & 0xFFFF, sequence & 0xFFFF)
}

/// Decode one pipe-pair harness value.
pub(super) fn decode_pipe_pair_value(value: HarnessValue<PipePair, PipePairVm>) -> PipePair {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Decode one shared-memory mapping harness value.
pub(super) fn decode_mapping_value(
    value: HarnessValue<SharedMemoryMapping, SharedMemoryMappingVm>,
) -> SharedMemoryMapping {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}
