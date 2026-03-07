use super::{backend, basic, event, monitor, window};

/// Callback used by one affinity-dispatched display test case.
type DisplayAffinityCallback = fn();

/// Named display test case registered for affinity dispatch.
struct DisplayAffinityCase {
    /// The stable libtest path for this case.
    name: &'static str,
    /// The callable test entry point.
    callback: DisplayAffinityCallback,
}

/// The complete affinity-sensitive display case registry.
const DISPLAY_AFFINITY_CASES: &[DisplayAffinityCase] = display_affinity_cases!();

/// Run one registered display affinity case by stable libtest path.
pub(crate) fn run_affinity_case(case_name: &str) -> bool {
    let Some(case) = DISPLAY_AFFINITY_CASES
        .iter()
        .find(|registered_case| registered_case.name == case_name)
    else {
        return false;
    };

    (case.callback)();
    true
}
