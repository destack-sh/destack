use std::fs;

use destack_lsp_types as lsp;

use super::harness::{LspHarness, temp_root, uri_for_path};

/// LSP didOpen publishes diagnostics for the document.
#[tokio::test]
async fn test_lsp_did_open_publishes_diagnostics() {
    let root = temp_root("did_open");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let mut harness = LspHarness::new(root.clone()).await;

    let path = root.join("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block: diagnostics match the opened document
    assert_eq!(diagnostics.uri, uri);
    assert!(diagnostics.diagnostics.is_empty());
}

/// LSP didChange publishes updated diagnostics.
#[tokio::test]
async fn test_lsp_did_change_updates_diagnostics() {
    let root = temp_root("did_change");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let mut harness = LspHarness::new(root.clone()).await;

    let path = root.join("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;

    let initial = harness.next_diagnostics_for(&uri).await;

    harness
        .did_change(uri.clone(), "export const x: number = \"bad\";\n", 2)
        .await;

    let updated = harness.next_diagnostics_for(&uri).await;

    // assertion block: diagnostics update for the same document
    assert_eq!(initial.uri, uri);
    assert_eq!(updated.uri, uri);
    assert!(updated.diagnostics.len() >= initial.diagnostics.len());
    assert!(!updated.diagnostics.is_empty());
}

/// LSP watched file changes publish diagnostics.
#[tokio::test]
async fn test_lsp_watched_file_change_publishes_diagnostics() {
    let root = temp_root("watched_change");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let path = root.join("main.ds");
    fs::write(&path, "export const x = ;\n").expect("write watched file");

    let mut harness = LspHarness::new(root.clone()).await;

    let uri = uri_for_path(&path);
    harness
        .did_change_watched(uri.clone(), lsp::FileChangeType::CHANGED)
        .await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block: diagnostics are reported for the changed file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didCreateFiles publishes diagnostics for new files.
#[tokio::test]
async fn test_lsp_did_create_publishes_diagnostics() {
    let root = temp_root("did_create");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let path = root.join("created.ds");
    fs::write(&path, "export const x = ;\n").expect("write created file");

    let mut harness = LspHarness::new(root.clone()).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block: diagnostics are reported for the created file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didDeleteFiles clears diagnostics for removed files.
#[tokio::test]
async fn test_lsp_did_delete_clears_diagnostics() {
    let root = temp_root("did_delete");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let path = root.join("deleted.ds");
    fs::write(&path, "export const x = ;\n").expect("write deleted file");

    let mut harness = LspHarness::new(root.clone()).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let created = harness.next_diagnostics_for(&uri).await;
    assert_eq!(created.uri, uri);
    assert!(!created.diagnostics.is_empty());

    fs::remove_file(&path).expect("remove deleted file");

    harness.did_delete(uri.clone()).await;

    let deleted = harness.next_diagnostics_for(&uri).await;

    // assertion block: diagnostics clear after delete
    assert_eq!(deleted.uri, uri);
    assert!(deleted.diagnostics.is_empty());
}

/// LSP didRenameFiles clears old diagnostics and publishes new ones.
#[tokio::test]
async fn test_lsp_did_rename_updates_diagnostics() {
    let root = temp_root("did_rename");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let old_path = root.join("old.ds");
    let new_path = root.join("new.ds");
    fs::write(&old_path, "export const x = ;\n").expect("write old file");

    let mut harness = LspHarness::new(root.clone()).await;

    let old_uri = uri_for_path(&old_path);
    let new_uri = uri_for_path(&new_path);
    harness.did_create(old_uri.clone()).await;

    let created = harness.next_diagnostics_for(&old_uri).await;
    assert_eq!(created.uri, old_uri);
    assert!(!created.diagnostics.is_empty());

    fs::rename(&old_path, &new_path).expect("rename file");

    harness.did_rename(old_uri.clone(), new_uri.clone()).await;

    let mut diagnostics = Vec::new();
    diagnostics.push(harness.next_diagnostics().await);
    diagnostics.push(harness.next_diagnostics().await);

    // assertion block: old is cleared, new has diagnostics
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
    let root = temp_root("watch_register");
    fs::create_dir_all(&root).expect("create lsp temp root");

    let mut harness = LspHarness::new(root.clone()).await;

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

    // assertion block: expected watch patterns are registered
    assert!(patterns.iter().any(|pattern| pattern == "**/*.ds"));
    assert!(patterns.iter().any(|pattern| pattern == "**/dsconfig.json"));
    assert!(
        patterns
            .iter()
            .any(|pattern| pattern == "**/tsconfig*.json")
    );
}
