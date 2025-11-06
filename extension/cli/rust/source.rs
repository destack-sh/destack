use std::fs;
use std::path::Path;

use dyst_source::{File, FileId, FileType, Uri};

use destack_terminal::CommandArguments;

/// Read a source either from a file or inline string argument.
pub(crate) fn get_file_from_arguments(ctx: &CommandArguments) -> Result<File, String> {
    if let Some(path) = ctx.option("file") {
        let path_ref = Path::new(path);
        fs::read_to_string(path_ref)
            .map_err(|error| format!("failed to read {path}: {error}"))
            .map(|content| {
                let name = path_ref
                    .iter()
                    .next_back()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or("<file>".to_string());
                let uri = Uri::from_string(path);
                File::from_string(FileId::new(0), name, uri, FileType::Dyst, content)
            })
    } else if let Some(string) = ctx.option("string") {
        Ok(File::from_string(
            FileId::new(0),
            "<string>".to_string(),
            Uri::from_string("<string>"),
            FileType::Dyst,
            string.to_string(),
        ))
    } else {
        Err("provide --file <path> or --string <string>".to_string())
    }
}
