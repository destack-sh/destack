use std::fs;
use std::path::Path;

use dyst_source::{File, FileId, FileType, Uri};

use destack_terminal::CommandArguments;

/// Read a source either from a file or inline string argument.
pub(crate) fn get_string_or_file(ctx: &CommandArguments) -> Result<Option<File>, String> {
    let format = FileType::from_extension_or_unknown(ctx.option("format").unwrap_or("ds"));
    let extension = format.extension().unwrap();

    // file
    if let Some(path_str) = ctx.option("file") {
        let path = Path::new(path_str);
        if path.extension().unwrap_or_default() != extension {
            return Err(format!("{path_str}: invalid file extension, expected {extension}"));
        }
        let name = path
            .iter()
            .next_back()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or("<file>".to_string());
        let uri = Uri::from_string(path_str);
        match fs::read_to_string(path) {
            Ok(content) => {
                let file = File::from_string(FileId::new(0), name, uri, format, content);
                Ok(Some(file))
            }
            Err(e) => Err(format!("\"{path_str}\": {e}")),
        }
    }
    // string
    else if let Some(string) = ctx.option("string") {
        let file = File::from_string(
            FileId::new(0),
            "<string>".to_string(),
            Uri::from_string("<string>"),
            format,
            string.to_string(),
        );
        Ok(Some(file))
    }
    // nothing
    else {
        Ok(None)
    }
}
