use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::BackendSupport;
use tracing::warn;

/// The strongest support rank seen in one selector lane.
const AVAILABLE_SUPPORT_RANK: u8 = 0;
/// The support rank for host integrations that exist but are not reachable.
const HOST_UNAVAILABLE_SUPPORT_RANK: u8 = 1;
/// The support rank for host integrations that were compiled out of this build.
const DISABLED_BY_BUILD_SUPPORT_RANK: u8 = 2;
/// The support rank for host integrations that do not exist on this target.
const UNSUPPORTED_TARGET_SUPPORT_RANK: u8 = 3;

impl BackendSupport {
    /// Return whether the backend integration is available.
    pub(crate) fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Convert one backend availability check result into one support state.
pub(crate) fn backend_support_from_check<T>(
    backend_name: &str,
    check: RuntimeResult<T>,
) -> BackendSupport {
    match check {
        Ok(_) => BackendSupport::Available,
        Err(error) => {
            warn!(
                backend = backend_name,
                error = %error,
                "backend availability check failed"
            );
            BackendSupport::HostUnavailable
        }
    }
}

/// Return the precedence rank for one backend support state.
fn backend_support_rank(support: BackendSupport) -> u8 {
    match support {
        BackendSupport::Available => AVAILABLE_SUPPORT_RANK,
        BackendSupport::HostUnavailable => HOST_UNAVAILABLE_SUPPORT_RANK,
        BackendSupport::DisabledByBuild => DISABLED_BY_BUILD_SUPPORT_RANK,
        BackendSupport::UnsupportedTarget => UNSUPPORTED_TARGET_SUPPORT_RANK,
    }
}

/// Aggregate support across one backend-selection lane.
pub(crate) fn aggregate_backend_support(
    supports: impl IntoIterator<Item = BackendSupport>,
) -> BackendSupport {
    // start from the weakest selector support state
    let mut aggregate = BackendSupport::UnsupportedTarget;

    for support in supports {
        // keep the strongest support state seen so far
        if backend_support_rank(support) < backend_support_rank(aggregate) {
            aggregate = support;
        }

        // stop once one backend lane is actually usable
        if support == BackendSupport::Available {
            break;
        }
    }

    aggregate
}

/// Build one not-supported error from backend support state.
pub(crate) fn backend_support_error(
    operation: &'static str,
    backend_name: &str,
    support: BackendSupport,
) -> Box<RuntimeError> {
    let message = match support {
        BackendSupport::Available => {
            format!("{operation}: backend {backend_name} is available for this host")
        }
        BackendSupport::UnsupportedTarget => {
            format!("{operation}: backend {backend_name} is not supported on this target")
        }
        BackendSupport::DisabledByBuild => {
            format!("{operation}: backend {backend_name} is disabled in this runtime build")
        }
        BackendSupport::HostUnavailable => {
            format!("{operation}: backend {backend_name} is not currently available on this host")
        }
    };

    RuntimeError::from(PlatformError::not_supported(message)).boxed()
}

#[cfg(test)]
mod tests {
    use super::{BackendSupport, aggregate_backend_support};

    /// Prefer availability whenever one candidate backend is usable.
    #[test]
    fn test_aggregate_backend_support_prefers_available() {
        let support = aggregate_backend_support([
            BackendSupport::DisabledByBuild,
            BackendSupport::Available,
            BackendSupport::HostUnavailable,
        ]);

        assert_eq!(support, BackendSupport::Available);
    }

    /// Prefer host-unavailable when no candidate backend is usable yet one host lane exists.
    #[test]
    fn test_aggregate_backend_support_prefers_host_unavailable_over_build_disable() {
        let support = aggregate_backend_support([
            BackendSupport::DisabledByBuild,
            BackendSupport::HostUnavailable,
            BackendSupport::UnsupportedTarget,
        ]);

        assert_eq!(support, BackendSupport::HostUnavailable);
    }

    /// Fall back to disabled-by-build when no supported host lane is reachable.
    #[test]
    fn test_aggregate_backend_support_prefers_build_disable_over_unsupported_target() {
        let support = aggregate_backend_support([
            BackendSupport::UnsupportedTarget,
            BackendSupport::DisabledByBuild,
        ]);

        assert_eq!(support, BackendSupport::DisabledByBuild);
    }

    /// Report unsupported-target when no candidate lane exists for the host.
    #[test]
    fn test_aggregate_backend_support_reports_unsupported_target_for_empty_lane() {
        let support = aggregate_backend_support([]);

        assert_eq!(support, BackendSupport::UnsupportedTarget);
    }
}
