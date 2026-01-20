use std::path::PathBuf;

use crate::{FormatterOptions, LinterOptions};

/// Options for configuring session defaults.
#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// Cache directory override (CLI or embedding).
    pub cache_dir_override: Option<PathBuf>,
}
