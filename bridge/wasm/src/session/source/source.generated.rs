// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::Edit;

/// Source input used to open a live session.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Source {
    content: SourceContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum SourceContent {
    /// Filesystem source rooted at a path.
    FileSystem {
        /// Source root or child path.
        path: String,
    },
    /// In-memory filesystem source seeded by edits.
    Memory {
        /// Source root path used for repository identity.
        root: String,
        /// Edits used to seed the memory filesystem.
        edits: Vec<Edit>,
    },
}

#[wasm_bindgen]
impl Source {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "fileSystem")]
    pub fn file_system(path: String) -> Self {
        Self {
            content: SourceContent::FileSystem { path },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "memory")]
    pub fn memory(root: String, edits: Vec<Edit>) -> Self {
        Self {
            content: SourceContent::Memory { root, edits },
        }
    }
}

impl Source {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::Source {
        match self.content {
            SourceContent::FileSystem { path } => bridge::Source::FileSystem { path },
            SourceContent::Memory { root, edits } => bridge::Source::Memory {
                root,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }
}
