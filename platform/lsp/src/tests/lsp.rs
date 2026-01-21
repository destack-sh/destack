use destack_lsp_types as lsp;
use destack_source::FileSystem;

use super::harness::{harness_for_fs, test_fs, uri_for_path};

/// LSP didOpen publishes diagnostics for the document.
#[tokio::test]
async fn test_lsp_did_open_publishes_diagnostics() {
    let fs = test_fs("did_open");
    let mut harness = harness_for_fs(&fs).await;

    let path = fs.path_for("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics match the opened document
    assert_eq!(diagnostics.uri, uri);
    assert!(diagnostics.diagnostics.is_empty());
}

/// LSP didChange publishes updated diagnostics.
#[tokio::test]
async fn test_lsp_did_change_updates_diagnostics() {
    let fs = test_fs("did_change");
    let mut harness = harness_for_fs(&fs).await;

    let path = fs.path_for("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;

    let initial = harness.next_diagnostics_for(&uri).await;

    harness
        .did_change(uri.clone(), "export const x: number = \"bad\";\n", 2)
        .await;

    let updated = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics update for the same document
    assert_eq!(initial.uri, uri);
    assert_eq!(updated.uri, uri);
    assert!(updated.diagnostics.len() >= initial.diagnostics.len());
    assert!(!updated.diagnostics.is_empty());
}

/// LSP watched file changes publish diagnostics.
#[tokio::test]
async fn test_lsp_watched_file_change_publishes_diagnostics() {
    let fs = test_fs("watched_change");
    let path = fs.write_text("main.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness
        .did_change_watched(uri.clone(), lsp::FileChangeType::CHANGED)
        .await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics are reported for the changed file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didCreateFiles publishes diagnostics for new files.
#[tokio::test]
async fn test_lsp_did_create_publishes_diagnostics() {
    let fs = test_fs("did_create");
    let path = fs.write_text("created.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics are reported for the created file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didDeleteFiles clears diagnostics for removed files.
#[tokio::test]
async fn test_lsp_did_delete_clears_diagnostics() {
    let fs = test_fs("did_delete");
    let path = fs.write_text("deleted.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let created = harness.next_diagnostics_for(&uri).await;
    assert_eq!(created.uri, uri);
    assert!(!created.diagnostics.is_empty());

    fs.remove_file(&path).unwrap();

    harness.did_delete(uri.clone()).await;

    let deleted = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics clear after delete
    assert_eq!(deleted.uri, uri);
    assert!(deleted.diagnostics.is_empty());
}

/// LSP didRenameFiles clears old diagnostics and publishes new ones.
#[tokio::test]
async fn test_lsp_did_rename_updates_diagnostics() {
    let fs = test_fs("did_rename");
    let old_path = fs.write_text("old.ds", "export const x = ;\n").unwrap();
    let new_path = fs.path_for("new.ds");

    let mut harness = harness_for_fs(&fs).await;

    let old_uri = uri_for_path(&old_path);
    let new_uri = uri_for_path(&new_path);
    harness.did_create(old_uri.clone()).await;

    let created = harness.next_diagnostics_for(&old_uri).await;
    assert_eq!(created.uri, old_uri);
    assert!(!created.diagnostics.is_empty());

    fs.rename(&old_path, &new_path).unwrap();

    harness.did_rename(old_uri.clone(), new_uri.clone()).await;

    let mut diagnostics = Vec::new();
    diagnostics.push(harness.next_diagnostics().await);
    diagnostics.push(harness.next_diagnostics().await);

    // check that the old is cleared, new has diagnostics
    assert_eq!(diagnostics.len(), 2);
    let old_diagnostics = diagnostics.iter().find(|diag| diag.uri == old_uri);
    let new_diagnostics = diagnostics.iter().find(|diag| diag.uri == new_uri);
    let old_diagnostics = old_diagnostics.expect("missing old diagnostics");
    let new_diagnostics = new_diagnostics.expect("missing new diagnostics");
    assert!(old_diagnostics.diagnostics.is_empty());
    assert!(!new_diagnostics.diagnostics.is_empty());
}

/// LSP registers file watchers during initialization.
#[tokio::test]
async fn test_lsp_registers_file_watchers() {
    let fs = test_fs("watch_register");
    let mut harness = harness_for_fs(&fs).await;

    let params = harness.next_register_capability().await;
    let registration = params
        .registrations
        .iter()
        .find(|registration| registration.method == "workspace/didChangeWatchedFiles")
        .expect("missing watched files registration");
    let options = registration
        .register_options
        .clone()
        .expect("missing watch registration options");

    let watchers = options
        .get("watchers")
        .and_then(|value| value.as_array())
        .expect("missing watchers");
    let patterns: Vec<String> = watchers
        .iter()
        .filter_map(|watcher| watcher.get("globPattern").and_then(|value| value.as_str()))
        .map(|pattern| pattern.to_string())
        .collect();

    // check that the expected watch patterns are registered
    assert!(patterns.iter().any(|pattern| pattern == "**/*.ds"));
    assert!(patterns.iter().any(|pattern| pattern == "**/dsconfig.json"));
    assert!(
        patterns
            .iter()
            .any(|pattern| pattern == "**/tsconfig*.json")
    );
}

/// LSP config changes publish diagnostics for invalidated modules.
#[tokio::test]
async fn test_lsp_config_change_fanout_publishes_diagnostics() {
    // set up the workspace root with a package and dsconfig
    let fs = test_fs("config_fanout");
    let _package_path = fs
        .write_text(
            "package.json",
            "{ \"name\": \"fanout\", \"version\": \"0.1.0\" }\n",
        )
        .unwrap();
    let dsconfig_path = fs
        .write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n")
        .unwrap();

    // write modules with invalid syntax
    let module_a = fs.write_text("a.ds", "export const a = ;\n").unwrap();
    let module_b = fs.write_text("b.ds", "export const b = ;\n").unwrap();

    // initialize the server
    let mut harness = harness_for_fs(&fs).await;

    // open both modules to register them with the daemon
    let uri_a = uri_for_path(&module_a);
    let uri_b = uri_for_path(&module_b);
    harness
        .did_open(uri_a.clone(), "export const a = ;\n")
        .await;
    harness
        .did_open(uri_b.clone(), "export const b = ;\n")
        .await;

    // open the dsconfig for edits
    let dsconfig_uri = uri_for_path(&dsconfig_path);
    harness
        .did_open(dsconfig_uri.clone(), "{ \"compilerOptions\": {} }\n")
        .await;

    // drain initial diagnostics from didOpen
    let _ = harness.next_diagnostics_for(&uri_a).await;
    let _ = harness.next_diagnostics_for(&uri_b).await;
    let _ = harness.next_diagnostics_for(&dsconfig_uri).await;

    // update dsconfig to trigger module invalidation
    fs.write_text(
        &dsconfig_path,
        "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
    )
    .unwrap();
    harness
        .did_change(
            dsconfig_uri.clone(),
            "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
            2,
        )
        .await;

    // diagnostics are published for both modules
    let diagnostics_a = harness.next_diagnostics_for(&uri_a).await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert_eq!(diagnostics_a.uri, uri_a);
    assert_eq!(diagnostics_b.uri, uri_b);
}
