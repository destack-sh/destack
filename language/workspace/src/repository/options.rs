use std::path::PathBuf;

use crate::{FormatterOptions, LinterOptions};

/// Options for configuring repository defaults.
#[derive(Debug, Clone, Default)]
pub struct RepositoryOptions {
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// Cache directory override.
    pub cache_dir_override: Option<PathBuf>,
}
