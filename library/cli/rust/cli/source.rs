use std::fs;
use std::path::Path;

use dyst_language_source::{Source, SourceId, Uri};

use crate::console::parse::CommandArguments;

/// Read a source either from a file or inline string argument.
pub(crate) fn read_source(ctx: &CommandArguments) -> Result<Source, String> {
    // --file
    if let Some(path) = ctx.option("file") {
        let path_ref = Path::new(path);
        fs::read_to_string(path_ref)
            .map_err(|error| format!("failed to read {path}: {error}"))
            .map(|content| Source::from_string(SourceId::new(0), Uri::from_string(path), content))
    }
    // --string
    else if let Some(string) = ctx.option("string") {
        Ok(Source::from_string(
            SourceId::new(0),
            Uri::from_string("<string>"),
            string.to_string(),
        ))
    }
    // no source provided
    else {
        Err("provide --file <path> or --string <string>".to_string())
    }
}
