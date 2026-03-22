use serde_json::Value;

/// Shared external source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceLocation {
    /// Load from one local file.
    File,
    /// Read from ambient process environment.
    ProcessEnvironment,
    /// Load from one external store reference.
    #[default]
    Remote,
}

/// Shared source selector options.
#[derive(Debug, Clone, Default)]
pub struct SourceSelectorOptions {
    /// Optional selected key within the source.
    pub key: Option<String>,
    /// Optional key prefix within the source.
    pub prefix: Option<String>,
}

impl SourceSelectorOptions {
    /// Inherit unset selector settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.key.is_none() {
            self.key = parent.key.clone();
        }

        if self.prefix.is_none() {
            self.prefix = parent.prefix.clone();
        }
    }
}

/// Shared source options.
#[derive(Debug, Clone, Default)]
pub struct SourceOptions<F> {
    /// Source location.
    pub location: SourceLocation,
    /// Location specific source locator.
    ///
    /// Use a file path for `file` sources or one store reference for `remote` sources.
    pub locator: Option<String>,
    /// Optional selected projection within the source.
    pub selector: SourceSelectorOptions,
    /// Declared source format.
    pub format: F,
    /// Whether missing source material is allowed.
    pub optional: bool,
    /// Extra source arguments.
    pub with: Option<Value>,
}

impl<F> SourceOptions<F>
where
    F: Copy + Default + PartialEq,
{
    /// Inherit unset source settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.locator.is_none() {
            self.locator = parent.locator.clone();
        }

        self.selector.extend_from(&parent.selector);

        if self.format == F::default() {
            self.format = parent.format;
        }

        self.optional = self.optional || parent.optional;

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}
