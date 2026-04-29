use im::OrdMap;

use destack_artifact::HostEnvironmentKey;

/// Host-provided inputs captured for one repository revision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct HostEnvironment {
    /// The captured process environment variables.
    pub env: OrdMap<String, String>,
}

impl HostEnvironment {
    /// Capture host inputs from the current process.
    pub fn capture_process() -> Self {
        let mut env = OrdMap::new();

        // process environment
        for (name, value) in std::env::vars() {
            env.insert(name, value);
        }

        Self { env }
    }

    /// Return one captured environment variable by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.env.get(name).map(String::as_str)
    }

    /// Return the captured environment entries in stable order.
    pub fn env_entries(&self) -> Vec<(String, String)> {
        self.env
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// Return one profile host environment key for all captured environment variables.
    pub fn key_all(&self) -> HostEnvironmentKey {
        HostEnvironmentKey::all(self.env_entries())
    }

    /// Return one profile host environment key for the requested environment variables.
    pub fn key_whitelist(&self, keys: &[String]) -> HostEnvironmentKey {
        HostEnvironmentKey::whitelist(keys, |key| self.get(key).map(ToOwned::to_owned))
    }
}
