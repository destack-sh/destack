use destack_source::{FileId, Span};

use crate::Session;

/// A code lens (inline annotation with optional command).
#[derive(Debug, Clone)]
pub struct CodeLens {
    /// The range this lens applies to.
    pub range: Span,
    /// The lens data.
    pub data: CodeLensData,
}

/// The data/command for a code lens.
#[derive(Debug, Clone)]
pub enum CodeLensData {
    /// Show reference count: "N references"
    References {
        /// Number of references (excluding declaration).
        count: usize,
    },
    /// Show implementation count: "N implementations"
    Implementations {
        /// Number of implementations.
        count: usize,
    },
    /// Run test action.
    RunTest {
        /// The test name.
        test_name: String,
    },
    /// Debug test action.
    DebugTest {
        /// The test name.
        test_name: String,
    },
    /// Custom lens with title and command.
    Custom {
        /// The title to display.
        title: String,
        /// The command identifier.
        command: String,
        /// Command arguments (JSON-serializable).
        arguments: Vec<String>,
    },
}

impl CodeLens {
    /// Create a references code lens.
    pub fn references(range: Span, count: usize) -> Self {
        Self {
            range,
            data: CodeLensData::References { count },
        }
    }

    /// Create an implementations code lens.
    pub fn implementations(range: Span, count: usize) -> Self {
        Self {
            range,
            data: CodeLensData::Implementations { count },
        }
    }

    /// Create a run test code lens.
    pub fn run_test(range: Span, test_name: impl Into<String>) -> Self {
        Self {
            range,
            data: CodeLensData::RunTest {
                test_name: test_name.into(),
            },
        }
    }

    /// Get the display title for this lens.
    pub fn title(&self) -> String {
        match &self.data {
            CodeLensData::References { count } => {
                if *count == 1 {
                    "1 reference".to_string()
                } else {
                    format!("{count} references")
                }
            }
            CodeLensData::Implementations { count } => {
                if *count == 1 {
                    "1 implementation".to_string()
                } else {
                    format!("{count} implementations")
                }
            }
            CodeLensData::RunTest { test_name } => format!("▶ Run {test_name}"),
            CodeLensData::DebugTest { test_name } => format!("🐛 Debug {test_name}"),
            CodeLensData::Custom { title, .. } => title.clone(),
        }
    }
}

/// Get code lenses for a file.
///
/// Code lenses appear as inline annotations above functions, classes, etc.
/// Common uses: reference counts, "Run Test" buttons, implementation counts.
pub fn code_lenses(_session: &Session, _file: FileId) -> Vec<CodeLens> {
    // 1. walk the file's declarations
    // 2. for functions/methods: count references, add "N references" lens
    // 3. for interfaces/abstract classes: count implementations
    // 4. for test functions: add "Run Test" lens
    // 5. for main functions: add "Run" lens
    todo!("#Incomplete: code_lenses")
}

/// Resolve a code lens (compute its command if deferred).
///
/// Some lenses defer computation until the user hovers/clicks.
pub fn resolve_code_lens(_session: &Session, _lens: &CodeLens) -> CodeLens {
    // For now, lenses are fully resolved on creation
    // This hook exists for expensive computations that should be deferred
    todo!("#Incomplete: resolve_code_lens")
}
