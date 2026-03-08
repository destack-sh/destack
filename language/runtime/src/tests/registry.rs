#[cfg(feature = "affinity")]
use crate::platform::display::tests::affinity as display_affinity_tests;

/// Run one registered affinity case by stable test path.
pub(crate) fn run_affinity_case(case_name: &str) -> bool {
    if display_affinity_tests::run_affinity_case(case_name) {
        return true;
    }

    false
}
