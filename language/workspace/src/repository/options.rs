use std::path::PathBuf;

use crate::{FormatterOptions, LinterOptions};

/// Options for configuring repository defaults.
#[derive(Debug, Clone)]
pub struct RepositoryOptions {
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// Cache directory override.
    pub cache_directory_override: Option<PathBuf>,
    /// The number of recent file revisions to retain per retained head.
    pub file_history_limit: usize,
}

impl Default for RepositoryOptions {
    fn default() -> Self {
        Self {
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            cache_directory_override: None,
            file_history_limit: 32,
        }
    }
}
