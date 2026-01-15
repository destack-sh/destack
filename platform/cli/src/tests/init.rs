use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{FileSystem, MemoryFileSystem};

use crate::command::init::{InitArgs, Template, run};
use crate::common::{FileSystemOverride, ReportArgs};

/// Initializes a minimal project in memory.
#[test]
fn test_init_minimal_creates_dsconfig() {
    // setup
    let fs = Arc::new(MemoryFileSystem::new());
    let root = PathBuf::from("/test/init/minimal");
    let args = InitArgs {
        dir: Some(root.clone()),
        name: Some("minimal".to_string()),
        template: Template::Minimal,
        force: false,
        fs_override: Some(FileSystemOverride::new(fs.clone())),
        report: ReportArgs::default(),
    };

    // run init
    let code = run(&args);

    // assert dsconfig exists
    assert_eq!(code, 0);
    assert!(fs.exists(&root.join("dsconfig.json")).unwrap_or(false));
}

/// Initializes an app template with the entry file.
#[test]
fn test_init_app_creates_entry() {
    // setup
    let fs = Arc::new(MemoryFileSystem::new());
    let root = PathBuf::from("/test/init/app");
    let args = InitArgs {
        dir: Some(root.clone()),
        name: Some("app".to_string()),
        template: Template::App,
        force: false,
        fs_override: Some(FileSystemOverride::new(fs.clone())),
        report: ReportArgs::default(),
    };

    // run init
    let code = run(&args);

    // assert entry file exists
    assert_eq!(code, 0);
    assert!(fs.exists(&root.join("src/main.ds")).unwrap_or(false));
}
