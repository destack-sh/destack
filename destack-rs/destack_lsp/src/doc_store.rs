use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tower_lsp::lsp_types as lsp;

/// Thread-safe in-memory document store.
#[derive(Debug, Default, Clone)]
pub struct DocumentStore(Arc<RwLock<HashMap<String, String>>>);

impl DocumentStore {
    /// Set full text for a document URI.
    pub fn set(&self, uri: &lsp::Url, text: String) {
        if let Ok(mut map) = self.0.write() {
            map.insert(uri.to_string(), text);
        }
    }

    /// Get full text for a document URI if available.
    pub fn get(&self, uri: &lsp::Url) -> Option<String> {
        self.0
            .read()
            .ok()
            .and_then(|m| m.get(&uri.to_string()).cloned())
    }
}
