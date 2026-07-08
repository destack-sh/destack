use serde::{Deserialize, Serialize};

use super::ObservationCategory;

/// Filter options for observation queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationOptions {
    /// Include runtime observations.
    pub runtime: bool,
    /// Include topology observations.
    pub topology: bool,
    /// Include resource observations.
    pub resource: bool,
    /// Include scheduler observations.
    pub scheduler: bool,
    /// Include diagnostic observations.
    pub diagnostic: bool,
    /// Include telemetry observations.
    pub telemetry: bool,
    /// Include domain observations.
    pub domain: bool,
}

impl Default for ObservationOptions {
    fn default() -> Self {
        Self {
            runtime: true,
            topology: true,
            resource: true,
            scheduler: true,
            diagnostic: true,
            telemetry: true,
            domain: true,
        }
    }
}

impl ObservationOptions {
    /// Return whether this filter allows one observation category.
    pub const fn allows(self, category: ObservationCategory) -> bool {
        match category {
            ObservationCategory::Runtime => self.runtime,
            ObservationCategory::Topology => self.topology,
            ObservationCategory::Resource => self.resource,
            ObservationCategory::Scheduler => self.scheduler,
            ObservationCategory::Diagnostic => self.diagnostic,
            ObservationCategory::Telemetry => self.telemetry,
            ObservationCategory::Domain => self.domain,
        }
    }
}
