use std::sync::Arc;

use destack_source::{FileType, FileWatchFilter, FileWatchOptions};

/// Build source file watch options.
pub fn source_watch_options() -> FileWatchOptions {
    let filter: FileWatchFilter = Arc::new(is_source_path);

    FileWatchOptions {
        filter: Some(filter),
        ..Default::default()
    }
}

/// Return whether a path should trigger workspace watch processing.
fn is_source_path(path: &std::path::Path) -> bool {
    let Some(file_type) = FileType::from_path(path) else {
        return false;
    };

    file_type.is_code() || file_type.is_data() || file_type.is_text()
}
