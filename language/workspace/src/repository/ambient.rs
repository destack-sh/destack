use im::OrdMap;

use destack_artifact::EnvironmentStamp;

/// One captured ambient semantic snapshot for one revision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AmbientSnapshot {
    /// The captured environment bindings.
    pub environment: EnvironmentSnapshot,
}

impl AmbientSnapshot {
    /// Capture one ambient snapshot from the current process.
    pub fn capture_process() -> Self {
        Self {
            environment: EnvironmentSnapshot::capture_process(),
        }
    }
}

/// One captured environment binding set for semantic evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EnvironmentSnapshot {
    /// The captured variable values by name.
    pub values: OrdMap<String, String>,
}

impl EnvironmentSnapshot {
    /// Capture one environment snapshot from the current process.
    pub fn capture_process() -> Self {
        let mut values = OrdMap::new();

        // process environment
        for (name, value) in std::env::vars() {
            values.insert(name, value);
        }

        Self { values }
    }

    /// Return one captured value by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// Return the captured entries in stable order.
    pub fn entries(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// Return one profile identity environment stamp for all captured values.
    pub fn environment_stamp_all(&self) -> EnvironmentStamp {
        EnvironmentStamp::from_entries_all(self.entries())
    }

    /// Return one profile identity environment stamp for the requested whitelist.
    pub fn environment_stamp_whitelist(&self, keys: &[String]) -> EnvironmentStamp {
        EnvironmentStamp::from_values_whitelist(keys, |key| self.get(key).map(ToOwned::to_owned))
    }
}
