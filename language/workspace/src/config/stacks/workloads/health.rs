use serde::Deserialize;

/// Health configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackHealthOptions {
    /// Startup probe.
    pub startup: StackProbeOptions,
    /// Readiness probe.
    pub readiness: StackProbeOptions,
    /// Liveness probe.
    pub liveness: StackProbeOptions,
}

impl StackHealthOptions {
    /// Inherit unset health settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        self.startup.extend_from(&parent.startup);
        self.readiness.extend_from(&parent.readiness);
        self.liveness.extend_from(&parent.liveness);
    }
}

impl From<&StackHealthJson> for StackHealthOptions {
    fn from(json: &StackHealthJson) -> Self {
        Self {
            startup: StackProbeOptions::from(&json.startup),
            readiness: StackProbeOptions::from(&json.readiness),
            liveness: StackProbeOptions::from(&json.liveness),
        }
    }
}

/// Probe configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackProbeOptions {
    /// Probe path.
    pub path: Option<String>,
    /// Probe HTTP method.
    pub method: Option<String>,
    /// Probe target port.
    pub port: Option<u16>,
    /// Initial delay in seconds.
    pub initial_delay_seconds: Option<u64>,
    /// Probe interval in seconds.
    pub interval_seconds: Option<u64>,
    /// Probe timeout in seconds.
    pub timeout_seconds: Option<u64>,
    /// Success threshold before state change.
    pub success_threshold: Option<u64>,
    /// Failure threshold before state change.
    pub failure_threshold: Option<u64>,
}

impl StackProbeOptions {
    /// Inherit unset probe settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.path.is_none() {
            self.path = parent.path.clone();
        }
        if self.method.is_none() {
            self.method = parent.method.clone();
        }
        if self.port.is_none() {
            self.port = parent.port;
        }
        if self.initial_delay_seconds.is_none() {
            self.initial_delay_seconds = parent.initial_delay_seconds;
        }
        if self.interval_seconds.is_none() {
            self.interval_seconds = parent.interval_seconds;
        }
        if self.timeout_seconds.is_none() {
            self.timeout_seconds = parent.timeout_seconds;
        }
        if self.success_threshold.is_none() {
            self.success_threshold = parent.success_threshold;
        }
        if self.failure_threshold.is_none() {
            self.failure_threshold = parent.failure_threshold;
        }
    }
}

impl From<&StackProbeJson> for StackProbeOptions {
    fn from(json: &StackProbeJson) -> Self {
        Self {
            path: json.path.clone(),
            method: json.method.clone(),
            port: json.port,
            initial_delay_seconds: json.initial_delay_seconds,
            interval_seconds: json.interval_seconds,
            timeout_seconds: json.timeout_seconds,
            success_threshold: json.success_threshold,
            failure_threshold: json.failure_threshold,
        }
    }
}

/// Health configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackHealthJson {
    /// Startup probe.
    #[serde(default)]
    pub startup: StackProbeJson,
    /// Readiness probe.
    #[serde(default)]
    pub readiness: StackProbeJson,
    /// Liveness probe.
    #[serde(default)]
    pub liveness: StackProbeJson,
}

/// Probe configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackProbeJson {
    /// Probe path.
    pub path: Option<String>,
    /// Probe HTTP method.
    pub method: Option<String>,
    /// Probe target port.
    pub port: Option<u16>,
    /// Initial delay in seconds.
    pub initial_delay_seconds: Option<u64>,
    /// Probe interval in seconds.
    pub interval_seconds: Option<u64>,
    /// Probe timeout in seconds.
    pub timeout_seconds: Option<u64>,
    /// Success threshold before state change.
    pub success_threshold: Option<u64>,
    /// Failure threshold before state change.
    pub failure_threshold: Option<u64>,
}
