
use std::collections::HashMap;
use std::sync::RwLock;

use dyst_language_ast::Parser;

use crate::protocol::types::Uri;

/// Maintain document Sources in a workspace. 
#[derive(Debug, Default, Clone)]
pub struct Analyzer<'a> {
    parsers_by_uri: HashMap<String, Parser<'a>>,
}

impl<'a> Analyzer<'a> {
    
}
