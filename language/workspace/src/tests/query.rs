use destack_repository::Revision;
use futures::executor::block_on;

use crate::tests::harness::TestWorkspace;
use crate::{DiagnosticsRequest, Error, RevisionPolicy, RunQueryInput};

/// Requires an explicit semantic target when resolving a query file.
#[test]
fn test_resolve_query_file_requires_target() {
    let test = TestWorkspace::new("query-file-target");
    let config_source = "{ \"name\": \"test\" }\n";
    let config = test.write_text("destack.json", config_source);
    let _ = test.apply_text(&config, config_source);
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let _ = test.apply_text(&path, source);
    let root = test.workspace.root_at(&path).expect("workspace root");

    let error = test
        .workspace
        .resolve_query_file(&root, path)
        .expect_err("query file without target should fail");

    assert!(matches!(error, Error::TargetNotSelected { .. }));
}

/// Executes latest and historical queries against their exact selected revisions.
#[test]
fn test_run_query_selects_exact_revision() {
    let test = TestWorkspace::new("query-exact-revision");
    let config_source = query_config();
    let config = test.write_text("destack.json", config_source);
    let _ = test.apply_text(&config, config_source);
    let path = test.write_text("main.ds", "export const value = 1;\n");
    let _ = test.apply_text(&path, "export const value = 1;\n");
    let first = test.workspace.revision_at(&path).expect("first revision");
    let _ = test.apply_text(&path, "export const value = 2;\n");
    let latest = test.workspace.revision_at(&path).expect("latest revision");
    let root = test.workspace.root_at(&path).expect("workspace root");

    // execute one continuation against its historical immutable revision
    let exact = block_on(test.workspace.run_query(
        &root,
        RunQueryInput {
            revision: RevisionPolicy::Exact(first),
            request: empty_rename_files(),
        },
    ))
    .expect("exact query");

    // execute one fresh program query against the current root head
    let current = block_on(test.workspace.run_query(
        &root,
        RunQueryInput {
            revision: RevisionPolicy::Latest,
            request: empty_rename_files(),
        },
    ))
    .expect("latest query");

    assert_eq!(exact.revision, first);
    assert_eq!(current.revision, latest);
    assert_ne!(first, latest);
}

/// Require matching revision preconditions for selected queries.
#[test]
fn test_run_query_requires_matching_revision() {
    let test = TestWorkspace::new("workspace_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.apply_text(&path, source);
    let current_revision = test
        .workspace
        .revision_at(&path)
        .expect("expected current revision");

    // reject stale revision preconditions
    let root = test
        .workspace
        .root_at(&path)
        .expect("expected workspace root");
    let stale_error = block_on(test.workspace.run_query(
        &root,
        RunQueryInput {
            revision: RevisionPolicy::Current(Revision::NULL),
            request: empty_rename_files(),
        },
    ))
    .expect_err("expected stale revision error");
    assert!(matches!(stale_error, Error::StaleRevision { .. }));

    // accept matching revision preconditions
    let _ = block_on(test.workspace.run_query(
        &root,
        RunQueryInput {
            revision: RevisionPolicy::Current(current_revision),
            request: empty_rename_files(),
        },
    ))
    .expect("expected query with matching revision");
}

/// Reads diagnostics from each exact source revision.
#[test]
#[ignore]
fn test_diagnose_successive_source_revisions() {
    let test = TestWorkspace::new("diagnose-source-revisions");
    let config_source = query_config();
    let config = test.write_text("destack.json", config_source);
    let _ = test.apply_text(&config, config_source);
    let first_source = r#"export function run(): void {
  const value: float64 = false;
}
"#;
    let path = test.write_text("main.ds", first_source);
    let _ = test.apply_text(&path, first_source);
    let first_revision = test
        .workspace
        .revision_at(&path)
        .expect("first diagnostic revision");

    // read the complete diagnostic result for the first revision
    let first = block_on(
        test.workspace
            .diagnose(DiagnosticsRequest::File(path.clone())),
    )
    .expect("first diagnostics");
    let first_diagnostics = first[0]
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.id.as_str(), diagnostic.message.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(first[0].revision, first_revision);
    assert_eq!(
        first_diagnostics,
        vec![(
            "not-assignable",
            "type 'false' is not assignable to type 'float64'"
        )]
    );

    // replace the source and require diagnostics from only the new revision
    let second_source = r#"export function run(): void {
  const value: float64 = 1;
}
"#;
    let _ = test.apply_text(&path, second_source);
    let second_revision = test
        .workspace
        .revision_at(&path)
        .expect("second diagnostic revision");
    let second = block_on(test.workspace.diagnose(DiagnosticsRequest::File(path)))
        .expect("second diagnostics");
    assert_ne!(first_revision, second_revision);
    assert_eq!(second[0].revision, second_revision);
    assert_eq!(second[0].diagnostics, Vec::new());
}

/// Builds one empty file-rename query for revision selection exercises.
fn empty_rename_files() -> destack_query::QueryRequest {
    destack_query::QueryRequest::RenameFiles(destack_query::RenameFilesRequest {
        renames: Vec::new(),
    })
}

/// Return one query configuration with an explicit semantic target.
fn query_config() -> &'static str {
    r#"{
  "name": "test",
  "targets": {
    "default": {
      "entry": ["main.ds"]
    }
  },
  "defaultTarget": "default"
}
"#
}
