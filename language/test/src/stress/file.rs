use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write one complete file through a unique sibling path.
pub(super) fn write_complete_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    let temporary_path = unique_sibling_path(path, "tmp")?;

    // write replacement content away from the target path
    if let Err(error) = fs::write(&temporary_path, content) {
        let _ = fs::remove_file(&temporary_path);

        return Err(format!(
            "failed to write {}: {error}",
            temporary_path.display()
        ));
    }

    // replace the target path in one filesystem operation
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);

        return Err(format!("failed to write {}: {error}", path.display()));
    }

    Ok(())
}

/// Return one unique sibling path for a temporary stress file.
pub(super) fn unique_sibling_path(path: &Path, suffix: &str) -> Result<PathBuf, String> {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return Err(format!(
            "stress path has no utf-8 file name: {}",
            path.display()
        ));
    };

    let process_id = std::process::id();
    let file_index = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary_name = format!("{file_name}.{process_id}.{file_index}.{suffix}");

    Ok(path.with_file_name(temporary_name))
}
