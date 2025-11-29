use std::path::Path;
use std::sync::Arc;

use destack_dir::Program;
use destack_source::{File, FileType, Uri};

/// Input arguments describing a source file or inline string.
pub(crate) struct SourceArg<'a> {
    /// The file path.
    pub file: Option<&'a str>,
    /// The inline string.
    pub string: Option<&'a str>,
    /// The source format.
    pub format: Option<&'a str>,
}

/// Read a source either from a file or inline string argument.
pub(crate) fn get_string_or_file(
    program: &Program,
    source: SourceArg<'_>,
) -> Result<Arc<File>, String> {
    let format_name = source.format.unwrap_or("ds");
    let format = FileType::from_extension_or_unknown(format_name);
    let extension = format.extension().unwrap();

    // file
    if let Some(path_str) = source.file {
        let path = Path::new(path_str);
        if path.extension().unwrap_or_default() != extension {
            return Err(format!(
                "{path_str}: invalid file extension, expected {extension}"
            ));
        }
        let file_id = program.files.next_id();
        let name = path
            .iter()
            .next_back()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or("<file>".to_string());
        let uri = Uri::from_string(path_str);
        match program.fs.read_to_string(path) {
            Ok(content) => {
                let file = File::from_text(
                    file_id,
                    name,
                    uri,
                    Some(path.to_path_buf()),
                    format,
                    content,
                );
                program.files.insert(file);
                Ok(program.files.get(file_id))
            }
            Err(e) => Err(format!("\"{path_str}\": {e}")),
        }
    }
    // string
    else if let Some(string) = source.string {
        let file_id = program.files.next_id();
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            format,
            string.to_string(),
        );
        program.files.insert(file);
        Ok(program.files.get(file_id))
    }
    // nothing
    else {
        Err("no source input provided".to_string())
    }
}
