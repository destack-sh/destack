use serde::Deserialize;

/// Timeout configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackTimeoutOptions {
    /// Maximum time to become ready after start.
    pub startup: Option<String>,
    /// Maximum time allowed for one health check.
    pub health: Option<String>,
    /// Maximum time allowed for one request.
    pub request: Option<String>,
    /// Maximum graceful shutdown interval.
    pub shutdown: Option<String>,
    /// Maximum runtime for one bounded job.
    pub job: Option<String>,
    /// Maximum time for one external or host operation.
    pub operation: Option<String>,
}

impl StackTimeoutOptions {
    /// Inherit unset timeout settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.startup.is_none() {
            self.startup = parent.startup.clone();
        }
        if self.health.is_none() {
            self.health = parent.health.clone();
        }
        if self.request.is_none() {
            self.request = parent.request.clone();
        }
        if self.shutdown.is_none() {
            self.shutdown = parent.shutdown.clone();
        }
        if self.job.is_none() {
            self.job = parent.job.clone();
        }
        if self.operation.is_none() {
            self.operation = parent.operation.clone();
        }
    }
}

impl From<&StackTimeoutJson> for StackTimeoutOptions {
    fn from(json: &StackTimeoutJson) -> Self {
        Self {
            startup: json.startup.clone(),
            health: json.health.clone(),
            request: json.request.clone(),
            shutdown: json.shutdown.clone(),
            job: json.job.clone(),
            operation: json.operation.clone(),
        }
    }
}

/// Timeout configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackTimeoutJson {
    /// Maximum time to become ready after start.
    pub startup: Option<String>,
    /// Maximum time allowed for one health check.
    pub health: Option<String>,
    /// Maximum time allowed for one request.
    pub request: Option<String>,
    /// Maximum graceful shutdown interval.
    pub shutdown: Option<String>,
    /// Maximum runtime for one bounded job.
    pub job: Option<String>,
    /// Maximum time for one external or host operation.
    pub operation: Option<String>,
}
