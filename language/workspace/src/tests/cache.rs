use tspp_artifact::BuildId;
use tspp_repository::TraceLevel;
use tspp_source::{Edit, FileSystem};

use crate::CommandRevision;

use super::harness::TestWorkspace;

/// Reuse an exact checked revision after replacing every live toolchain owner.
#[test]
fn test_restart_reuses_exact_artifacts() {
    let test = TestWorkspace::persistent("artifact-cache-restart");
    let config_source = r#"{
  "name": "test",
  "targets": {
    "default": {
      "entry": ["main.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;
    let config = test.write_text("destack.json", config_source);
    test.apply_text(&config, config_source);
    let main_source = "export const answer: int32 = 42;\n";
    let main = test.write_text("main.tspp", main_source);
    test.apply_text(&main, main_source);

    // build and persist one complete physical check
    let first = test.check(main.clone(), CommandRevision::Current);
    assert!(
        first
            .trace
            .as_ref()
            .is_some_and(|trace| trace.stats.built > 0)
    );
    test.save_artifacts();
    let cache = test.artifact_cache().to_path_buf();
    let build = cache.join("builds").join(BuildId::test().to_string());
    let mut packs = test
        .fs
        .read_dir(&build.join("packs"))
        .expect("read initial artifact packs");
    packs.sort_unstable();
    let manifests = test
        .fs
        .read_dir(&build.join("manifests"))
        .expect("read initial artifact manifests");
    let [manifest] = manifests.as_slice() else {
        panic!("expected one artifact cache manifest, found {manifests:?}");
    };
    let manifest = test.fs.read(manifest).expect("read initial manifest");
    assert!(cache.join("lock").is_file());
    assert!(cache.join("last-collection").is_file());
    let revision = test.workspace.revision().expect("read physical revision");
    let is_scheduled = test
        .workspace
        .persist_artifacts(revision)
        .expect("schedule unchanged artifact selection");
    assert!(!is_scheduled);

    // replace the host, repository, table, stores, session, and executor
    let test = test.restart();
    let revision = test.workspace.revision().expect("read restored revision");
    let is_scheduled = test
        .workspace
        .persist_artifacts(revision)
        .expect("schedule restored artifact selection");
    assert!(!is_scheduled);
    let second = test.check(main.clone(), CommandRevision::Current);
    let trace = second.trace.expect("restored check trace");

    assert_eq!(first.diagnostics, second.diagnostics);
    assert_eq!(trace.stats.built, 0);
    assert_eq!(trace.stats.failed, 0);
    // persist and restore the same selection again
    test.save_artifacts();
    let mut current_packs = test
        .fs
        .read_dir(&build.join("packs"))
        .expect("read retained artifact packs");
    current_packs.sort_unstable();
    let current_manifests = test
        .fs
        .read_dir(&build.join("manifests"))
        .expect("read retained artifact manifests");
    let [current_manifest] = current_manifests.as_slice() else {
        panic!("expected one artifact cache manifest, found {current_manifests:?}");
    };
    let current_manifest = test
        .fs
        .read(current_manifest)
        .expect("read retained manifest");
    assert_eq!(current_packs, packs);
    assert_eq!(current_manifest, manifest);

    let test = test.restart();
    let third = test.check(main, CommandRevision::Current);
    let trace = third.trace.expect("second restored check trace");

    assert_eq!(trace.stats.built, 0);
    assert_eq!(trace.stats.failed, 0);
}

/// Rebuild changed source artifacts while reusing the retained repository selection.
#[test]
fn test_restart_invalidates_changed_source() {
    let test = TestWorkspace::persistent("artifact-cache-edit");
    let main_source = "export struct Position {}\n";
    let main = test.write_text("main.tspp", main_source);
    test.apply_text(&main, main_source);

    // persist one complete source generation
    let first = test.check(main.clone(), CommandRevision::Current);
    let first = first.trace.expect("initial check trace");
    test.save_artifacts();

    // replace physical source without evaluating the new revision
    let changed_source = "/// A simple position.\nexport struct Position {}\n";
    test.apply_text(&main, changed_source);
    let test = test.restart();
    let second = test.check(main.clone(), CommandRevision::Current);
    let second = second.trace.expect("incremental check trace");
    let built = second
        .attempts
        .iter()
        .filter(|attempt| attempt.outcome == "built")
        .map(|attempt| attempt.name.as_str())
        .collect::<Vec<_>>();

    assert!(second.stats.built < first.stats.built);
    assert_eq!(
        built,
        [
            "dir.parse",
            "dir.bind",
            "dir.import",
            "dir.expand",
            "dir.export",
            "dir.resolve",
            "dir.declare",
            "dir.elaborate",
            "dir.check",
        ]
    );
    assert_eq!(second.stats.failed, 0);

    // persist and restore the changed generation without another rebuild
    test.save_artifacts();
    let test = test.restart();
    let restored = test.check(main, CommandRevision::Current);
    let restored = restored.trace.expect("restored changed trace");

    assert_eq!(restored.stats.built, 0);
    assert_eq!(restored.stats.failed, 0);
}

/// Keep branch artifacts out of the physical repository manifest.
#[test]
fn test_restart_ignores_branch_selection() {
    let test = TestWorkspace::persistent("artifact-cache-branch");
    let main_source = "export const answer: int32 = 42;\n";
    let main = test.write_text("main.tspp", main_source);
    test.apply_text(&main, main_source);

    // persist the complete physical selection
    test.check(main.clone(), CommandRevision::Current);
    test.save_artifacts();
    let physical = test.workspace.revision().expect("read physical revision");

    // evaluate a private branch without publishing it as physical state
    let branch = "cache-test".to_string();
    test.workspace
        .create_branch(branch.clone(), physical)
        .expect("create branch");
    let trace = test.workspace.start_trace(TraceLevel::Disabled);
    let commit = test
        .workspace
        .edit_branch(
            &branch,
            physical,
            vec![Edit::SetText {
                path: main.clone(),
                text: "export const answer: boolean = true;\n".to_string(),
            }],
            &trace,
        )
        .expect("edit branch");
    test.check(main.clone(), CommandRevision::Exact(commit.after));
    test.workspace
        .flush_artifact_cache()
        .expect("flush artifact cache");

    // restore the unchanged physical revision without branch artifacts
    let test = test.restart();
    let restored = test.check(main, CommandRevision::Current);
    let trace = restored.trace.expect("restored physical check trace");

    assert_eq!(trace.stats.built, 0);
    assert_eq!(trace.stats.failed, 0);
}
