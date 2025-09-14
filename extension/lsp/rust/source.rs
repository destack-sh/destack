use std::collections::HashMap;
use std::sync::RwLock;

use dyst_language_source::Source;

use crate::protocol::types::Uri;

/// Maintain document Sources in a workspace. 
#[derive(Debug, Default, Clone)]
pub struct SourceStore {
    source_id_counter: u32,
    sources_by_uri: HashMap<String, Source>,
}

impl SourceStore {
    
}
