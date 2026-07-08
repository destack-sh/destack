use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::ObservationCategory;

/// Runtime observation filter bit.
const RUNTIME: u8 = 1 << 0;
/// Topology observation filter bit.
const TOPOLOGY: u8 = 1 << 1;
/// Resource observation filter bit.
const RESOURCE: u8 = 1 << 2;
/// Scheduler observation filter bit.
const SCHEDULER: u8 = 1 << 3;
/// Diagnostic observation filter bit.
const DIAGNOSTIC: u8 = 1 << 4;
/// Telemetry observation filter bit.
const TELEMETRY: u8 = 1 << 5;
/// Domain observation filter bit.
const DOMAIN: u8 = 1 << 6;
/// All observation filter bits.
const ALL: u8 = RUNTIME | TOPOLOGY | RESOURCE | SCHEDULER | DIAGNOSTIC | TELEMETRY | DOMAIN;

/// Filter options for observation queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationOptions {
    /// Included observation categories.
    mask: u8,
}

/// Serialized observation-options record.
#[derive(Deserialize)]
struct ObservationOptionsRecord {
    /// Include runtime observations.
    runtime: bool,
    /// Include topology observations.
    topology: bool,
    /// Include resource observations.
    resource: bool,
    /// Include scheduler observations.
    scheduler: bool,
    /// Include diagnostic observations.
    diagnostic: bool,
    /// Include telemetry observations.
    telemetry: bool,
    /// Include domain observations.
    domain: bool,
}

impl Serialize for ObservationOptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ObservationOptions", 7)?;
        state.serialize_field("runtime", &self.allows(ObservationCategory::Runtime))?;
        state.serialize_field("topology", &self.allows(ObservationCategory::Topology))?;
        state.serialize_field("resource", &self.allows(ObservationCategory::Resource))?;
        state.serialize_field("scheduler", &self.allows(ObservationCategory::Scheduler))?;
        state.serialize_field("diagnostic", &self.allows(ObservationCategory::Diagnostic))?;
        state.serialize_field("telemetry", &self.allows(ObservationCategory::Telemetry))?;
        state.serialize_field("domain", &self.allows(ObservationCategory::Domain))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ObservationOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let record = ObservationOptionsRecord::deserialize(deserializer)?;
        let mut mask = 0;
        if record.runtime {
            mask |= RUNTIME;
        }
        if record.topology {
            mask |= TOPOLOGY;
        }
        if record.resource {
            mask |= RESOURCE;
        }
        if record.scheduler {
            mask |= SCHEDULER;
        }
        if record.diagnostic {
            mask |= DIAGNOSTIC;
        }
        if record.telemetry {
            mask |= TELEMETRY;
        }
        if record.domain {
            mask |= DOMAIN;
        }

        Ok(Self { mask })
    }
}

impl Default for ObservationOptions {
    fn default() -> Self {
        Self::all()
    }
}

impl ObservationOptions {
    /// Include every observation category.
    pub const fn all() -> Self {
        Self { mask: ALL }
    }

    /// Return whether this filter allows one observation category.
    pub const fn allows(self, category: ObservationCategory) -> bool {
        self.mask & category.mask() != 0
    }
}

impl ObservationCategory {
    /// Return the filter bit for this category.
    const fn mask(self) -> u8 {
        match self {
            Self::Runtime => RUNTIME,
            Self::Topology => TOPOLOGY,
            Self::Resource => RESOURCE,
            Self::Scheduler => SCHEDULER,
            Self::Diagnostic => DIAGNOSTIC,
            Self::Telemetry => TELEMETRY,
            Self::Domain => DOMAIN,
        }
    }
}
