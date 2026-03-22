use indexmap::IndexMap;
use serde::Deserialize;

use super::common::{
    StackAccessMode, StackAccessModeJson, StackCacheJson, StackCacheOptions, StackIssuerRefJson,
    StackIssuerRefOptions, merge_metadata,
};

/// Stack ingress configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackIngressOptions {
    /// Named service routes.
    pub routes: IndexMap<String, StackRouteOptions>,
    /// Named redirect rules.
    pub redirects: IndexMap<String, StackRedirectOptions>,
    /// Named rewrite rules.
    pub rewrites: IndexMap<String, StackRewriteOptions>,
    /// Named header rules.
    pub headers: IndexMap<String, StackHeaderRuleOptions>,
    /// CORS defaults.
    pub cors: StackCorsOptions,
}

impl StackIngressOptions {
    /// Inherit unset ingress settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        for (name, route) in &parent.routes {
            if let Some(current) = self.routes.get_mut(name) {
                current.extend_from(route);
            } else {
                self.routes.insert(name.clone(), route.clone());
            }
        }

        for (name, redirect) in &parent.redirects {
            if let Some(current) = self.redirects.get_mut(name) {
                current.extend_from(redirect);
            } else {
                self.redirects.insert(name.clone(), redirect.clone());
            }
        }

        for (name, rewrite) in &parent.rewrites {
            if let Some(current) = self.rewrites.get_mut(name) {
                current.extend_from(rewrite);
            } else {
                self.rewrites.insert(name.clone(), rewrite.clone());
            }
        }

        for (name, header) in &parent.headers {
            if let Some(current) = self.headers.get_mut(name) {
                current.extend_from(header);
            } else {
                self.headers.insert(name.clone(), header.clone());
            }
        }

        self.cors.extend_from(&parent.cors);
    }
}

impl From<&StackIngressJson> for StackIngressOptions {
    fn from(json: &StackIngressJson) -> Self {
        Self {
            routes: json
                .routes
                .as_ref()
                .map(|routes| {
                    routes
                        .iter()
                        .map(|(name, route)| (name.clone(), StackRouteOptions::from(route)))
                        .collect()
                })
                .unwrap_or_default(),
            redirects: json
                .redirects
                .as_ref()
                .map(|redirects| {
                    redirects
                        .iter()
                        .map(|(name, redirect)| {
                            (name.clone(), StackRedirectOptions::from(redirect))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            rewrites: json
                .rewrites
                .as_ref()
                .map(|rewrites| {
                    rewrites
                        .iter()
                        .map(|(name, rewrite)| (name.clone(), StackRewriteOptions::from(rewrite)))
                        .collect()
                })
                .unwrap_or_default(),
            headers: json
                .headers
                .as_ref()
                .map(|headers| {
                    headers
                        .iter()
                        .map(|(name, header)| (name.clone(), StackHeaderRuleOptions::from(header)))
                        .collect()
                })
                .unwrap_or_default(),
            cors: StackCorsOptions::from(&json.cors),
        }
    }
}

/// Route configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackRouteOptions {
    /// Referenced domain name.
    pub domain: Option<String>,
    /// Match pattern.
    pub r#match: Option<String>,
    /// Target service name.
    pub service: Option<String>,
    /// Target publication name.
    pub publication: Option<String>,
    /// Allowed HTTP methods.
    pub methods: Vec<String>,
    /// Route access expectations.
    pub access: StackRouteAccessOptions,
    /// Route policy.
    pub policy: StackRoutePolicyOptions,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
}

impl StackRouteOptions {
    /// Inherit unset route settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.domain.is_none() {
            self.domain = parent.domain.clone();
        }
        if self.r#match.is_none() {
            self.r#match = parent.r#match.clone();
        }
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.publication.is_none() {
            self.publication = parent.publication.clone();
        }
        if self.methods.is_empty() {
            self.methods = parent.methods.clone();
        }
        self.access.extend_from(&parent.access);
        self.policy.extend_from(&parent.policy);

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackRouteJson> for StackRouteOptions {
    fn from(json: &StackRouteJson) -> Self {
        Self {
            domain: json.domain.clone(),
            r#match: json.r#match.clone(),
            service: json.service.clone(),
            publication: json.publication.clone(),
            methods: json.methods.clone().unwrap_or_default(),
            access: StackRouteAccessOptions::from(&json.access),
            policy: StackRoutePolicyOptions::from(&json.policy),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
        }
    }
}

/// Route access options.
#[derive(Debug, Clone, Default)]
pub struct StackRouteAccessOptions {
    /// Access mode.
    pub mode: Option<StackAccessMode>,
    /// Accepted issuer reference.
    pub issuer: Option<StackIssuerRefOptions>,
    /// Accepted audience names.
    pub audiences: Vec<String>,
    /// Required scopes.
    pub scopes: Vec<String>,
    /// Required roles.
    pub roles: Vec<String>,
}

impl StackRouteAccessOptions {
    /// Inherit unset route access settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if let Some(parent_issuer) = &parent.issuer {
            if let Some(issuer) = self.issuer.as_mut() {
                issuer.extend_from(parent_issuer);
            } else {
                self.issuer = Some(parent_issuer.clone());
            }
        }
        if self.audiences.is_empty() {
            self.audiences = parent.audiences.clone();
        }
        if self.scopes.is_empty() {
            self.scopes = parent.scopes.clone();
        }
        if self.roles.is_empty() {
            self.roles = parent.roles.clone();
        }
    }
}

impl From<&StackRouteAccessJson> for StackRouteAccessOptions {
    fn from(json: &StackRouteAccessJson) -> Self {
        Self {
            mode: json.mode.map(StackAccessMode::from),
            issuer: json.issuer.as_ref().map(StackIssuerRefOptions::from),
            audiences: json.audiences.clone().unwrap_or_default(),
            scopes: json.scopes.clone().unwrap_or_default(),
            roles: json.roles.clone().unwrap_or_default(),
        }
    }
}

/// Route policy options.
#[derive(Debug, Clone, Default)]
pub struct StackRoutePolicyOptions {
    /// Route timeout budget.
    pub timeout: Option<String>,
    /// Service and publication resolution strategy.
    pub resolution: Option<StackRouteResolution>,
    /// Retry policy.
    pub retries: StackRouteRetryOptions,
    /// Cache policy.
    pub cache: StackCacheOptions,
}

impl StackRoutePolicyOptions {
    /// Inherit unset route policy settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.timeout.is_none() {
            self.timeout = parent.timeout.clone();
        }
        if self.resolution.is_none() {
            self.resolution = parent.resolution;
        }
        self.retries.extend_from(&parent.retries);
        self.cache.extend_from(&parent.cache);
    }
}

impl From<&StackRoutePolicyJson> for StackRoutePolicyOptions {
    fn from(json: &StackRoutePolicyJson) -> Self {
        Self {
            timeout: json.timeout.clone(),
            resolution: json.resolution.map(StackRouteResolution::from),
            retries: StackRouteRetryOptions::from(&json.retries),
            cache: StackCacheOptions::from(&json.cache),
        }
    }
}

/// Route resolution options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackRouteResolution {
    /// Resolve publications before service execution.
    PublicationFirst,
    /// Resolve service execution before publications.
    ServiceFirst,
}

impl From<StackRouteResolutionJson> for StackRouteResolution {
    fn from(json: StackRouteResolutionJson) -> Self {
        match json {
            StackRouteResolutionJson::PublicationFirst => Self::PublicationFirst,
            StackRouteResolutionJson::ServiceFirst => Self::ServiceFirst,
        }
    }
}

/// Route retry policy options.
#[derive(Debug, Clone, Default)]
pub struct StackRouteRetryOptions {
    /// Maximum retry attempts.
    pub attempts: Option<u64>,
    /// Timeout for one retry attempt.
    pub per_try_timeout: Option<String>,
    /// Retry conditions.
    pub conditions: Vec<String>,
}

impl StackRouteRetryOptions {
    /// Inherit unset route retry settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.attempts.is_none() {
            self.attempts = parent.attempts;
        }
        if self.per_try_timeout.is_none() {
            self.per_try_timeout = parent.per_try_timeout.clone();
        }
        if self.conditions.is_empty() {
            self.conditions = parent.conditions.clone();
        }
    }
}

impl From<&StackRouteRetryJson> for StackRouteRetryOptions {
    fn from(json: &StackRouteRetryJson) -> Self {
        Self {
            attempts: json.attempts,
            per_try_timeout: json.per_try_timeout.clone(),
            conditions: json.conditions.clone().unwrap_or_default(),
        }
    }
}

/// Redirect configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackRedirectOptions {
    /// Source match pattern.
    pub from: Option<String>,
    /// Redirect destination.
    pub to: Option<String>,
    /// HTTP status code.
    pub status: Option<u16>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
}

impl StackRedirectOptions {
    /// Inherit unset redirect settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.from.is_none() {
            self.from = parent.from.clone();
        }
        if self.to.is_none() {
            self.to = parent.to.clone();
        }
        if self.status.is_none() {
            self.status = parent.status;
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackRedirectJson> for StackRedirectOptions {
    fn from(json: &StackRedirectJson) -> Self {
        Self {
            from: json.from.clone(),
            to: json.to.clone(),
            status: json.status,
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
        }
    }
}

/// Rewrite configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackRewriteOptions {
    /// Source match pattern.
    pub from: Option<String>,
    /// Rewrite destination.
    pub to: Option<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
}

impl StackRewriteOptions {
    /// Inherit unset rewrite settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.from.is_none() {
            self.from = parent.from.clone();
        }
        if self.to.is_none() {
            self.to = parent.to.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackRewriteJson> for StackRewriteOptions {
    fn from(json: &StackRewriteJson) -> Self {
        Self {
            from: json.from.clone(),
            to: json.to.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
        }
    }
}

/// Header rule configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackHeaderRuleOptions {
    /// Match pattern.
    pub r#match: Option<String>,
    /// Headers to set.
    pub set: IndexMap<String, String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
}

impl StackHeaderRuleOptions {
    /// Inherit unset header rule settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.r#match.is_none() {
            self.r#match = parent.r#match.clone();
        }

        merge_metadata(&mut self.set, &parent.set);
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackHeaderRuleJson> for StackHeaderRuleOptions {
    fn from(json: &StackHeaderRuleJson) -> Self {
        Self {
            r#match: json.r#match.clone(),
            set: json.set.clone().unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
        }
    }
}

/// CORS configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackCorsOptions {
    /// Allowed origins.
    pub origins: Vec<String>,
    /// Allowed methods.
    pub methods: Vec<String>,
    /// Allowed request headers.
    pub allow_headers: Vec<String>,
    /// Exposed response headers.
    pub expose_headers: Vec<String>,
    /// Whether credentials are allowed.
    pub credentials: Option<bool>,
    /// Cache lifetime in seconds.
    pub max_age_seconds: Option<u64>,
}

impl StackCorsOptions {
    /// Inherit unset CORS settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.origins.is_empty() {
            self.origins = parent.origins.clone();
        }
        if self.methods.is_empty() {
            self.methods = parent.methods.clone();
        }
        if self.allow_headers.is_empty() {
            self.allow_headers = parent.allow_headers.clone();
        }
        if self.expose_headers.is_empty() {
            self.expose_headers = parent.expose_headers.clone();
        }
        if self.credentials.is_none() {
            self.credentials = parent.credentials;
        }
        if self.max_age_seconds.is_none() {
            self.max_age_seconds = parent.max_age_seconds;
        }
    }
}

impl From<&StackCorsJson> for StackCorsOptions {
    fn from(json: &StackCorsJson) -> Self {
        Self {
            origins: json.origins.clone().unwrap_or_default(),
            methods: json.methods.clone().unwrap_or_default(),
            allow_headers: json.allow_headers.clone().unwrap_or_default(),
            expose_headers: json.expose_headers.clone().unwrap_or_default(),
            credentials: json.credentials,
            max_age_seconds: json.max_age_seconds,
        }
    }
}

/// An external routing surface.
///
/// Inputs: domains, services, publications, and routing rules.
/// Outputs: externally reachable routes and edge policy.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackIngressJson {
    /// Named service or publication routes.
    pub routes: Option<IndexMap<String, StackRouteJson>>,
    /// Named redirect rules.
    pub redirects: Option<IndexMap<String, StackRedirectJson>>,
    /// Named rewrite rules.
    pub rewrites: Option<IndexMap<String, StackRewriteJson>>,
    /// Named header rules.
    pub headers: Option<IndexMap<String, StackHeaderRuleJson>>,
    /// CORS defaults.
    #[serde(default)]
    pub cors: StackCorsJson,
}

/// Route configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRouteJson {
    /// Referenced domain name.
    pub domain: Option<String>,
    /// Match pattern.
    pub r#match: Option<String>,
    /// Target service name.
    pub service: Option<String>,
    /// Target publication name.
    pub publication: Option<String>,
    /// Allowed HTTP methods.
    pub methods: Option<Vec<String>>,
    /// Route access expectations.
    #[serde(default)]
    pub access: StackRouteAccessJson,
    /// Route policy.
    #[serde(default)]
    pub policy: StackRoutePolicyJson,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
}

/// Route access JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRouteAccessJson {
    /// Access mode.
    pub mode: Option<StackAccessModeJson>,
    /// Accepted issuer reference.
    pub issuer: Option<StackIssuerRefJson>,
    /// Accepted audience names.
    pub audiences: Option<Vec<String>>,
    /// Required scopes.
    pub scopes: Option<Vec<String>>,
    /// Required roles.
    pub roles: Option<Vec<String>>,
}

/// Route policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRoutePolicyJson {
    /// Route timeout budget.
    pub timeout: Option<String>,
    /// Service and publication resolution strategy.
    pub resolution: Option<StackRouteResolutionJson>,
    /// Retry policy.
    #[serde(default)]
    pub retries: StackRouteRetryJson,
    /// Cache policy.
    #[serde(default)]
    pub cache: StackCacheJson,
}

/// Route resolution JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum StackRouteResolutionJson {
    /// Resolve publications before service execution.
    PublicationFirst,
    /// Resolve service execution before publications.
    ServiceFirst,
}

/// Route retry policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRouteRetryJson {
    /// Maximum retry attempts.
    pub attempts: Option<u64>,
    /// Timeout for one retry attempt.
    pub per_try_timeout: Option<String>,
    /// Retry conditions.
    pub conditions: Option<Vec<String>>,
}

/// Redirect configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRedirectJson {
    /// Source match pattern.
    pub from: Option<String>,
    /// Redirect destination.
    pub to: Option<String>,
    /// HTTP status code.
    pub status: Option<u16>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
}

/// Rewrite configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRewriteJson {
    /// Source match pattern.
    pub from: Option<String>,
    /// Rewrite destination.
    pub to: Option<String>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
}

/// Header rule configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackHeaderRuleJson {
    /// Match pattern.
    pub r#match: Option<String>,
    /// Headers to set.
    pub set: Option<IndexMap<String, String>>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
}

/// CORS configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackCorsJson {
    /// Allowed origins.
    pub origins: Option<Vec<String>>,
    /// Allowed methods.
    pub methods: Option<Vec<String>>,
    /// Allowed request headers.
    pub allow_headers: Option<Vec<String>>,
    /// Exposed response headers.
    pub expose_headers: Option<Vec<String>>,
    /// Whether credentials are allowed.
    pub credentials: Option<bool>,
    /// Cache lifetime in seconds.
    pub max_age_seconds: Option<u64>,
}
