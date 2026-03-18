use serde::Deserialize;
use serde_json::Value;

/// Asset publication options.
#[derive(Debug, Clone, Default)]
pub struct StackAssetsOptions {
    /// Local source directory or manifest path.
    pub source: Option<String>,
    /// Backing asset or CDN service reference.
    pub service: Option<String>,
    /// Public path prefix where assets are exposed.
    pub public_path: Option<String>,
    /// Cache-Control header for published assets.
    pub cache_control: Option<String>,
    /// Whether published asset paths are content-addressed and immutable.
    pub immutable: Option<bool>,
    /// Default index document for directory requests.
    pub index_document: Option<String>,
    /// Default error document for not-found responses.
    pub error_document: Option<String>,
    /// Extra asset metadata.
    pub config: Option<Value>,
}

impl StackAssetsOptions {
    /// Inherit unset asset settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.source.is_none() {
            self.source = parent.source.clone();
        }
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.public_path.is_none() {
            self.public_path = parent.public_path.clone();
        }
        if self.cache_control.is_none() {
            self.cache_control = parent.cache_control.clone();
        }
        if self.immutable.is_none() {
            self.immutable = parent.immutable;
        }
        if self.index_document.is_none() {
            self.index_document = parent.index_document.clone();
        }
        if self.error_document.is_none() {
            self.error_document = parent.error_document.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
    }
}

impl From<&StackAssetsJson> for StackAssetsOptions {
    fn from(json: &StackAssetsJson) -> Self {
        match json {
            StackAssetsJson::Reference(source) => Self {
                source: Some(source.clone()),
                ..Default::default()
            },
            StackAssetsJson::Object(object) => Self {
                source: object.source.clone(),
                service: object.service.clone(),
                public_path: object.public_path.clone(),
                cache_control: object.cache_control.clone(),
                immutable: object.immutable,
                index_document: object.index_document.clone(),
                error_document: object.error_document.clone(),
                config: object.config.clone(),
            },
        }
    }
}

/// Asset publication JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum StackAssetsJson {
    /// Shorthand local source directory or manifest path.
    Reference(String),
    /// Structured asset publication settings.
    Object(StackAssetsObjectJson),
}

/// Structured asset publication JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackAssetsObjectJson {
    /// Local source directory or manifest path.
    pub source: Option<String>,
    /// Backing asset or CDN service reference.
    pub service: Option<String>,
    /// Public path prefix where assets are exposed.
    pub public_path: Option<String>,
    /// Cache-Control header for published assets.
    pub cache_control: Option<String>,
    /// Whether published asset paths are content-addressed and immutable.
    pub immutable: Option<bool>,
    /// Default index document for directory requests.
    pub index_document: Option<String>,
    /// Default error document for not-found responses.
    pub error_document: Option<String>,
    /// Extra asset metadata.
    pub config: Option<Value>,
}
