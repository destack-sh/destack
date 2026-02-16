use super::with_harness_context;
use crate::platform::process::ExecAtFlags;
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

#[cfg(unix)]
#[test]
fn test_process_exec_error_paths() {
    with_harness_context(|mut context| {
        let empty = Vec::<String>::new();

        let result = context.exec("/definitely/missing/destack-command", &empty, &empty);
        assert!(result.is_err());

        let missing_directory = DirectoryHandle(ResourceId(0));
        let missing_file = FileHandle(ResourceId(0));

        let execat = context.execat(missing_directory, "missing", &empty, &empty, ExecAtFlags(0));
        assert!(execat.is_err());

        let fexec = context.fexec(missing_file, &empty, &empty);
        assert!(fexec.is_err());

        Ok(())
    });
}
