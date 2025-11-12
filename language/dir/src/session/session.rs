use std::collections::HashMap;

use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, File, FileId, LanguageOptions};

use crate::ModuleRegistry;

/// A session for a language.
#[derive(Debug)]
pub struct Session {
    /// The language options.
    pub language: LanguageOptions,
    /// The files in the session.
    pub files: HashMap<FileId, File>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The combined string pool.
    pub strings: StringPool,
}
