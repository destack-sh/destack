use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Instant;

use tspp_artifact::ArtifactKey;
use tspp_source::TargetId;

use crate::tests::TestSession;

/// Check every builtin library module.
#[test]
fn test_check_library() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .unwrap_or_else(|error| panic!("builtin library target profile should resolve:\n{error}"))
        .id();

    // check every builtin module through the normal artifact path
    let keys = package
        .module_ids()
        .map(|module| ArtifactKey::dir_checked(module, profile))
        .collect::<Vec<_>>();
    let started = Instant::now();
    let result = session.require_all_traced(keys.iter().copied());
    let elapsed = started.elapsed();

    // report the modules' own diagnostics before the artifact failure
    let diagnostics = session.render_terminal_diagnostics_for(&keys);
    if !diagnostics.is_empty() {
        panic!("\n{diagnostics}");
    }
    if let Err(error) = result {
        panic!("{error}");
    }

    println!("checked {} library modules in {elapsed:.1?}", keys.len());
    session.print_trace("library", keys.len());

    // sum declare, elaborate, and check per module: the sema cost table
    let mut sema = BTreeMap::<String, u64>::new();
    for attempt in &session.trace().attempts {
        let is_sema = matches!(
            attempt.name.as_str(),
            "dir.declare" | "dir.elaborate" | "dir.check"
        );
        if is_sema && let Some(label) = &attempt.label {
            *sema.entry(label.clone()).or_default() += attempt.work_micros;
        }
    }
    // sum the check work counters per module
    let mut counters = BTreeMap::<String, BTreeMap<String, u64>>::new();
    for attempt in &session.trace().attempts {
        if attempt.name != "dir.check" {
            continue;
        }
        let Some(label) = &attempt.label else {
            continue;
        };
        let row = counters.entry(label.clone()).or_default();
        for counter in &attempt.counters {
            if counter.name.starts_with("check.") {
                *row.entry(counter.name.clone()).or_default() += counter.value;
            }
        }
    }

    // print the most expensive modules first
    let mut rows = sema.into_iter().collect::<Vec<_>>();
    rows.sort_by_key(|(_, micros)| Reverse(*micros));
    println!("\nsema work per module (declare + elaborate + check)");
    for (label, micros) in rows.iter().take(20) {
        println!("{:>9.3} ms  {label}", *micros as f64 / 1000.0);
        if let Some(row) = counters.get(label) {
            let row = row
                .iter()
                .map(|(name, value)| format!("{}={value}", &name["check.".len()..]))
                .collect::<Vec<_>>()
                .join(" ");
            println!("             {row}");
        }
    }
}
