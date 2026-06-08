use super::*;
use destack_repository::{RuntimeDiagnosticLevel, RuntimeDiagnosticOptions};

/// Record diagnostics at or above the configured severity threshold.
#[test]
fn test_record_filters_by_level() {
    let diagnostics = DiagnosticStore::from_options(&RuntimeDiagnosticOptions {
        level: RuntimeDiagnosticLevel::Warn,
        capacity: Some(8),
    });

    diagnostics.record(
        RuntimeDiagnosticLevel::Info,
        "display",
        "destack.display.window.open",
        "ignored diagnostic",
        None,
    );

    diagnostics.record(
        RuntimeDiagnosticLevel::Warn,
        "display",
        "destack.display.window.open",
        "recorded diagnostic",
        Some(5),
    );

    // inspect the stored entries directly
    let state = diagnostics.state.lock();
    let entries = state.diagnostic_entries.iter().cloned().collect::<Vec<_>>();
    drop(state);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, RuntimeDiagnosticLevel::Warn);
    assert_eq!(entries[0].module, "display");
    assert_eq!(entries[0].os_code, Some(5));
}

/// Keep diagnostics bounded to the configured ring buffer capacity.
#[test]
fn test_record_enforces_capacity_and_drops_oldest() {
    let diagnostics = DiagnosticStore::from_options(&RuntimeDiagnosticOptions {
        level: RuntimeDiagnosticLevel::Trace,
        capacity: Some(2),
    });

    diagnostics.record(
        RuntimeDiagnosticLevel::Error,
        "display",
        "op1",
        "message1",
        None,
    );

    diagnostics.record(
        RuntimeDiagnosticLevel::Warn,
        "display",
        "op2",
        "message2",
        None,
    );

    diagnostics.record(
        RuntimeDiagnosticLevel::Info,
        "display",
        "op3",
        "message3",
        None,
    );

    // inspect the stored entries directly
    let state = diagnostics.state.lock();
    let entries = state.diagnostic_entries.iter().cloned().collect::<Vec<_>>();
    let dropped_count = state.dropped_since_drain;
    drop(state);

    assert_eq!(dropped_count, 1);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].operation, "op2");
    assert_eq!(entries[1].operation, "op3");
}

/// Reuse freed runtime error slots with generation protection.
#[test]
fn test_record_error_reuses_slots_with_generation() {
    let diagnostics = DiagnosticStore::default();

    let first_id = diagnostics.record_error(
        RuntimeError::Internal {
            message: "first".to_string(),
        }
        .boxed(),
    );

    let taken = diagnostics.take_error(first_id).unwrap();
    assert_eq!(taken.message(), "internal error: first");
    assert!(diagnostics.take_error(first_id).is_none());

    let second_id = diagnostics.record_error(
        RuntimeError::Internal {
            message: "second".to_string(),
        }
        .boxed(),
    );

    assert_eq!(first_id.slot(), second_id.slot());
    assert_ne!(first_id.generation(), second_id.generation());
}
