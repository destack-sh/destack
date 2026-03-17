use indexmap::IndexMap;
use serde::Deserialize;

use super::common::merge_metadata;

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
    /// Allowed HTTP methods.
    pub methods: Vec<String>,
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
        if self.methods.is_empty() {
            self.methods = parent.methods.clone();
        }

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
            methods: json.methods.clone().unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
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
/// Inputs: domains, services, and routing rules.
/// Outputs: externally reachable routes and edge policy.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackIngressJson {
    /// Named service routes.
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
    /// Allowed HTTP methods.
    pub methods: Option<Vec<String>>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
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
