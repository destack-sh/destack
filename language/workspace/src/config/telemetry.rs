use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::stacks::merge_metadata;

/// Telemetry sink kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TelemetrySinkKind {
    /// OpenTelemetry OTLP sink.
    #[default]
    Otlp,
    /// Human readable stdout sink.
    Stdout,
    /// File based sink.
    File,
    /// External vendor sink.
    Vendor,
}

/// Reusable telemetry policy options.
#[derive(Debug, Clone, Default)]
pub struct TelemetryOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Resource attributes attached to all emitted observations.
    pub resource: IndexMap<String, String>,
    /// Observation policy for the runtime and libraries.
    pub observe: TelemetryObserveOptions,
    /// Named sinks.
    pub sinks: IndexMap<String, TelemetrySinkOptions>,
    /// Extra telemetry arguments.
    pub with: Option<Value>,
}

impl TelemetryOptions {
    /// Inherit unset telemetry settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        merge_metadata(&mut self.resource, &parent.resource);

        self.observe.extend_from(&parent.observe);

        for (name, sink) in &parent.sinks {
            if let Some(current) = self.sinks.get_mut(name) {
                current.extend_from(sink);
            } else {
                self.sinks.insert(name.clone(), sink.clone());
            }
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&TelemetryJson> for TelemetryOptions {
    fn from(json: &TelemetryJson) -> Self {
        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            resource: json.resource.clone().unwrap_or_default(),
            observe: json
                .observe
                .as_ref()
                .map(TelemetryObserveOptions::from)
                .unwrap_or_default(),
            sinks: json
                .sinks
                .as_ref()
                .map(|sinks| {
                    sinks
                        .iter()
                        .map(|(name, sink)| (name.clone(), TelemetrySinkOptions::from(sink)))
                        .collect()
                })
                .unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Telemetry observation options.
#[derive(Debug, Clone, Default)]
pub struct TelemetryObserveOptions {
    /// Observe runtime, resource, scheduler, and platform events.
    pub runtime: Option<bool>,
    /// Observe diagnostics, failures, and warnings.
    pub diagnostic: Option<bool>,
    /// Observe application and library domain events.
    pub domain: Option<bool>,
    /// Extra observation arguments.
    pub with: Option<Value>,
}

impl TelemetryObserveOptions {
    /// Inherit unset observation settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.runtime.is_none() {
            self.runtime = parent.runtime;
        }

        if self.diagnostic.is_none() {
            self.diagnostic = parent.diagnostic;
        }

        if self.domain.is_none() {
            self.domain = parent.domain;
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&TelemetryObserveJson> for TelemetryObserveOptions {
    fn from(json: &TelemetryObserveJson) -> Self {
        Self {
            runtime: json.runtime,
            diagnostic: json.diagnostic,
            domain: json.domain,
            with: json.with.clone(),
        }
    }
}

/// Telemetry sink options.
#[derive(Debug, Clone, Default)]
pub struct TelemetrySinkOptions {
    /// Sink kind.
    pub kind: Option<TelemetrySinkKind>,
    /// Sink endpoint.
    pub endpoint: Option<String>,
    /// Transport protocol.
    pub protocol: Option<String>,
    /// Static request headers.
    pub headers: IndexMap<String, String>,
    /// Extra exporter arguments.
    pub with: Option<Value>,
}

impl TelemetrySinkOptions {
    /// Inherit unset sink settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.kind.is_none() {
            self.kind = parent.kind;
        }

        if self.endpoint.is_none() {
            self.endpoint = parent.endpoint.clone();
        }

        if self.protocol.is_none() {
            self.protocol = parent.protocol.clone();
        }

        merge_metadata(&mut self.headers, &parent.headers);

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&TelemetrySinkJson> for TelemetrySinkOptions {
    fn from(json: &TelemetrySinkJson) -> Self {
        Self {
            kind: json.kind,
            endpoint: json.endpoint.clone(),
            protocol: json.protocol.clone(),
            headers: json.headers.clone().unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Telemetry reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TelemetryRefsJson {
    /// One named telemetry definition reference.
    One(String),
    /// Many named telemetry definition references.
    Many(Vec<String>),
}

impl TelemetryRefsJson {
    /// Return the referenced telemetry names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(name) => vec![name.clone()],
            Self::Many(names) => names.clone(),
        }
    }
}

/// Convert telemetry declarations into normalized options.
pub fn telemetry_options_from_json(
    json: &Option<IndexMap<String, TelemetryJson>>,
) -> IndexMap<String, TelemetryOptions> {
    json.as_ref()
        .map(|telemetry| {
            telemetry
                .iter()
                .map(|(name, entry)| (name.clone(), TelemetryOptions::from(entry)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one telemetry map from a parent config.
pub fn extend_telemetry_options(
    current: &mut IndexMap<String, TelemetryOptions>,
    parent: &IndexMap<String, TelemetryOptions>,
) {
    for (name, telemetry) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(telemetry);
        } else {
            current.insert(name.clone(), telemetry.clone());
        }
    }
}

/// Reusable telemetry policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TelemetryJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Resource attributes attached to all emitted observations.
    pub resource: Option<IndexMap<String, String>>,
    /// Observation policy for the runtime and libraries.
    pub observe: Option<TelemetryObserveJson>,
    /// Named sinks.
    pub sinks: Option<IndexMap<String, TelemetrySinkJson>>,
    /// Extra telemetry arguments.
    pub with: Option<Value>,
}

/// Telemetry observation JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TelemetryObserveJson {
    /// Observe runtime, resource, scheduler, and platform events.
    pub runtime: Option<bool>,
    /// Observe diagnostics, failures, and warnings.
    pub diagnostic: Option<bool>,
    /// Observe application and library domain events.
    pub domain: Option<bool>,
    /// Extra observation arguments.
    pub with: Option<Value>,
}

/// Telemetry sink JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySinkJson {
    /// Sink kind.
    pub kind: Option<TelemetrySinkKind>,
    /// Sink endpoint.
    pub endpoint: Option<String>,
    /// Transport protocol.
    pub protocol: Option<String>,
    /// Static request headers.
    pub headers: Option<IndexMap<String, String>>,
    /// Extra exporter arguments.
    pub with: Option<Value>,
}
