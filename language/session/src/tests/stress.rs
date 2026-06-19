use destack_repository::TraceSnapshot;

use super::TestSession;

/// Build one generated source tree fixture.
fn generated_source_tree(count: usize) -> Vec<(String, String)> {
    let mut files = vec![(
        "destack.json".to_string(),
        r#"{
  "name": "@test/app"
}
"#
        .to_string(),
    )];

    for index in 0..count {
        files.push((
            format!("src/generated/file-{index}.ds"),
            format!(
                r#"export const value{index} = {index};
"#
            ),
        ));
    }

    files
}

/// Attempt counts for one traced session operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TraceCounts {
    /// The total artifact attempts.
    artifacts: usize,
    /// The artifacts built by providers.
    built: usize,
    /// The artifacts reused from the in-memory table.
    memory_cached: usize,
    /// The artifacts restored from the artifact store.
    store_cached: usize,
    /// The attempts that parked on missing dependencies.
    parked: usize,
    /// The attempts that failed.
    failed: usize,
}

impl TraceCounts {
    /// Count artifact attempt outcomes in one trace.
    fn from_trace(trace: &TraceSnapshot) -> Self {
        let mut counts = Self {
            artifacts: trace.artifacts.len(),
            ..Self::default()
        };

        for artifact in &trace.artifacts {
            match artifact.outcome.as_str() {
                "built" => counts.built += 1,
                "memory_cached" => counts.memory_cached += 1,
                "store_cached" => counts.store_cached += 1,
                "parked" => counts.parked += 1,
                "failed" => counts.failed += 1,
                outcome => panic!("unknown artifact trace outcome {outcome}"),
            }
        }

        counts
    }
}

#[test]
fn test_open_tracks_large_source_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.assert_file_count(10_001);
}

#[test]
fn test_reload_reports_one_update_in_large_tree() {
    let files = generated_source_tree(10_000);
    let files = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let test = TestSession::open(&files).unwrap();

    test.write(
        "src/generated/file-500.ds",
        r#"export const v500 = 999;
"#,
    );
    let updates = test.reload();

    test.assert_update_paths(&updates, &["src/generated/file-500.ds"]);
    test.assert_file_count(10_001);
}

#[test]
fn test_check_reuses_unaffected_artifacts_after_single_edit() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"import { value } from "./dep";

export const result = value;
"#,
        ),
        (
            "src/dep.ds",
            r#"export const value = 1;
"#,
        ),
    ])
    .unwrap();

    let (cold, cold_trace) = test.check("src/index.ds", "js");
    assert_eq!(
        TraceCounts::from_trace(&cold_trace),
        TraceCounts {
            artifacts: 5081,
            built: 3203,
            memory_cached: 0,
            store_cached: 0,
            parked: 1878,
            failed: 0,
        },
    );

    let (warm, warm_trace) = test.check("src/index.ds", "js");
    assert_eq!(warm, cold);
    assert_eq!(TraceCounts::from_trace(&warm_trace), TraceCounts::default());

    test.edit_text(
        "src/dep.ds",
        r#"export const value = 2;
"#,
    );

    let (edited, edited_trace) = test.check("src/index.ds", "js");
    assert_ne!(edited, cold);
    assert_eq!(
        TraceCounts::from_trace(&edited_trace),
        TraceCounts {
            artifacts: 23,
            built: 12,
            memory_cached: 0,
            store_cached: 0,
            parked: 11,
            failed: 0,
        },
    );
}
