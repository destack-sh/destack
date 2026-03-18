use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{StackTlsMode, StackTlsModeJson, merge_metadata};

/// Stack domain options.
#[derive(Debug, Clone, Default)]
pub struct StackDomainOptions {
    /// Fully qualified domain name.
    pub name: Option<String>,
    /// Parent domain reference.
    pub parent: Option<String>,
    /// Domain ownership mode.
    pub mode: Option<StackDomainMode>,
    /// DNS management settings.
    pub dns: StackDomainDnsOptions,
    /// TLS management settings.
    pub tls: StackDomainTlsOptions,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
}

impl StackDomainOptions {
    /// Inherit unset domain settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.parent.is_none() {
            self.parent = parent.parent.clone();
        }
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }

        self.dns.extend_from(&parent.dns);
        self.tls.extend_from(&parent.tls);
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackDomainJson> for StackDomainOptions {
    fn from(json: &StackDomainJson) -> Self {
        Self {
            name: json.name.clone(),
            parent: json.parent.clone(),
            mode: json.mode.map(StackDomainMode::from),
            dns: StackDomainDnsOptions::from(&json.dns),
            tls: StackDomainTlsOptions::from(&json.tls),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
        }
    }
}

/// Domain DNS options.
#[derive(Debug, Clone, Default)]
pub struct StackDomainDnsOptions {
    /// DNS provider identifier.
    pub provider: Option<String>,
    /// Control-plane account reference.
    pub account: Option<String>,
    /// DNS zone identifier or name.
    pub zone: Option<String>,
    /// Explicit DNS records to manage.
    pub records: IndexMap<String, StackDomainDnsRecordOptions>,
    /// Extra DNS provider metadata.
    pub config: Option<Value>,
}

impl StackDomainDnsOptions {
    /// Inherit unset DNS settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.account.is_none() {
            self.account = parent.account.clone();
        }
        if self.zone.is_none() {
            self.zone = parent.zone.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }

        for (name, record) in &parent.records {
            if let Some(current) = self.records.get_mut(name) {
                current.extend_from(record);
            } else {
                self.records.insert(name.clone(), record.clone());
            }
        }
    }
}

impl From<&StackDomainDnsJson> for StackDomainDnsOptions {
    fn from(json: &StackDomainDnsJson) -> Self {
        Self {
            provider: json.provider.clone(),
            account: json.account.clone(),
            zone: json.zone.clone(),
            records: json
                .records
                .as_ref()
                .map(|records| {
                    records
                        .iter()
                        .map(|(name, record)| {
                            (name.clone(), StackDomainDnsRecordOptions::from(record))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            config: json.config.clone(),
        }
    }
}

/// Domain DNS record options.
#[derive(Debug, Clone, Default)]
pub struct StackDomainDnsRecordOptions {
    /// DNS record type.
    pub r#type: Option<String>,
    /// Relative record name within the zone.
    pub name: Option<String>,
    /// DNS record value.
    pub value: Option<String>,
    /// DNS record ttl in seconds.
    pub ttl: Option<u32>,
    /// DNS routing priority.
    pub priority: Option<u16>,
    /// Whether the record is proxied through the DNS provider.
    pub proxied: Option<bool>,
}

impl StackDomainDnsRecordOptions {
    /// Inherit unset DNS record settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.r#type.is_none() {
            self.r#type = parent.r#type.clone();
        }
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.value.is_none() {
            self.value = parent.value.clone();
        }
        if self.ttl.is_none() {
            self.ttl = parent.ttl;
        }
        if self.priority.is_none() {
            self.priority = parent.priority;
        }
        if self.proxied.is_none() {
            self.proxied = parent.proxied;
        }
    }
}

impl From<&StackDomainDnsRecordJson> for StackDomainDnsRecordOptions {
    fn from(json: &StackDomainDnsRecordJson) -> Self {
        Self {
            r#type: json.r#type.clone(),
            name: json.name.clone(),
            value: json.value.clone(),
            ttl: json.ttl,
            priority: json.priority,
            proxied: json.proxied,
        }
    }
}

/// Domain TLS options.
#[derive(Debug, Clone, Default)]
pub struct StackDomainTlsOptions {
    /// TLS mode.
    pub mode: Option<StackTlsMode>,
    /// Certificate reference.
    pub certificate: Option<String>,
}

impl StackDomainTlsOptions {
    /// Inherit unset TLS settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if self.certificate.is_none() {
            self.certificate = parent.certificate.clone();
        }
    }
}

impl From<&StackDomainTlsJson> for StackDomainTlsOptions {
    fn from(json: &StackDomainTlsJson) -> Self {
        Self {
            mode: json.mode.map(StackTlsMode::from),
            certificate: json.certificate.clone(),
        }
    }
}

/// Domain ownership mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackDomainMode {
    /// Destack manages the domain directly.
    #[default]
    Managed,
    /// Destack manages one delegated child domain.
    Delegated,
    /// Destack references an external unmanaged domain.
    External,
}

impl From<StackDomainModeJson> for StackDomainMode {
    fn from(json: StackDomainModeJson) -> Self {
        match json {
            StackDomainModeJson::Managed => Self::Managed,
            StackDomainModeJson::Delegated => Self::Delegated,
            StackDomainModeJson::External => Self::External,
        }
    }
}

/// Domain ownership mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackDomainModeJson {
    /// Destack manages the domain directly.
    Managed,
    /// Destack manages one delegated child domain.
    Delegated,
    /// Destack references an external unmanaged domain.
    External,
}

/// A domain ownership and DNS policy node.
///
/// Inputs: domain name, ownership mode, DNS settings, and TLS settings.
/// Outputs: hostname references and provider-managed DNS or TLS handles.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackDomainJson {
    /// Fully qualified domain name.
    pub name: Option<String>,
    /// Parent domain reference.
    pub parent: Option<String>,
    /// Domain ownership mode.
    pub mode: Option<StackDomainModeJson>,
    /// DNS management settings.
    #[serde(default)]
    pub dns: StackDomainDnsJson,
    /// TLS management settings.
    #[serde(default)]
    pub tls: StackDomainTlsJson,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
}

/// Domain DNS JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackDomainDnsJson {
    /// DNS provider identifier.
    pub provider: Option<String>,
    /// Control-plane account reference.
    pub account: Option<String>,
    /// DNS zone identifier or name.
    pub zone: Option<String>,
    /// Explicit DNS records to manage.
    pub records: Option<IndexMap<String, StackDomainDnsRecordJson>>,
    /// Extra DNS provider metadata.
    pub config: Option<Value>,
}

/// Domain DNS record JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackDomainDnsRecordJson {
    /// DNS record type.
    pub r#type: Option<String>,
    /// Relative record name within the zone.
    pub name: Option<String>,
    /// DNS record value.
    pub value: Option<String>,
    /// DNS record ttl in seconds.
    pub ttl: Option<u32>,
    /// DNS routing priority.
    pub priority: Option<u16>,
    /// Whether the record is proxied through the DNS provider.
    pub proxied: Option<bool>,
}

/// Domain TLS JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackDomainTlsJson {
    /// TLS mode.
    pub mode: Option<StackTlsModeJson>,
    /// Certificate reference.
    pub certificate: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StackDomainJson;

    /// Parse structured DNS records for one domain.
    #[test]
    fn test_domain_parse_dns_records() {
        let json: StackDomainJson = serde_json::from_value(json!({
            "name": "example.com",
            "dns": {
                "provider": "cloudflare",
                "records": {
                    "root": {
                        "type": "A",
                        "name": "@",
                        "value": "192.0.2.10",
                        "ttl": 60,
                        "proxied": true
                    },
                    "mail": {
                        "type": "MX",
                        "name": "@",
                        "value": "mail.example.com",
                        "priority": 10
                    }
                }
            }
        }))
        .expect("domain should parse");

        let records = json.dns.records.expect("records should be present");

        assert_eq!(records["root"].r#type.as_deref(), Some("A"));
        assert_eq!(records["root"].proxied, Some(true));
        assert_eq!(records["mail"].r#type.as_deref(), Some("MX"));
        assert_eq!(records["mail"].priority, Some(10));
    }
}
