use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Instant;

use destack_artifact::ArtifactKey;
use destack_source::TargetId;

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

/// Resolve intrinsic equality for builtin scalars and declared equality for library scalars.
#[test]
fn test_resolve_scalar_equality_protocols() {
    let session = TestSession::single(
        r#"
import { Equal, PartialEqual } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare function requirePartialEqual<T: PartialEqual<T>>(value: T): void;

declare const booleanValue: boolean;
declare const characterValue: char;
declare const integerValue: int32;
declare const floatValue: float64;
declare const symbolValue: symbol;
declare const stringValue: string;
declare const bigintValue: bigint;

requireEqual(booleanValue);
requireEqual(characterValue);
requireEqual(integerValue);
requirePartialEqual(floatValue);
requireEqual(symbolValue);
requireEqual(stringValue);
requireEqual(bigintValue);
requireEqual(true);
requireEqual('x');
requireEqual(1 as int32);
requirePartialEqual(1.0);
requireEqual(null);
requireEqual(undefined);
"#,
    );

    session.assert_dir_checked_diagnostics("main.ds", "");
}

/// Keep floating point equality partial because NaN is not equal to itself.
#[test]
fn test_reject_total_float_equality() {
    let session = TestSession::single(
        r#"
import { Equal } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare const value: float64;

requireEqual(value);
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'float64' does not satisfy 'Equal<float64>'"
/// @diagnostic.label line=7 column=1 span="requireEqual(value)" line_source="requireEqual(value);"
/// @diagnostic.related line=4 column=31 span="T" line_source="declare function requireEqual<T: Equal<T>>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

/// Reject cross type equality that has no declared implementation.
#[test]
fn test_reject_intrinsic_cross_type_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;
declare const value: int32;

requireStringEqual(value);
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'PartialEqual<string>'"
/// @diagnostic.label line=7 column=1 span="requireStringEqual(value)" line_source="requireStringEqual(value);"
/// @diagnostic.related line=4 column=37 span="T" line_source="declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

/// Resolve scalar and borrowed string membership through their matching protocols.
#[test]
fn test_resolve_collection_membership() {
    let session = TestSession::single(
        r#"
import { Array, Map, Set } from "destack:collections";
import { StringSlice } from "destack:string";

declare const integers: Array<int32>;
declare const integer: int32;
declare const floats: Array<float64>;
declare const float: float64;
declare const strings: Array<string>;
declare const stringValue: string;
declare const stringSlice: &readonly StringSlice;
declare const stringMap: Map<string, int32>;
declare const stringSet: Set<string>;

integers.includes(integer);
floats.includes(float);
strings.includes(stringValue);
strings.includes(stringSlice);
stringMap.has(stringSlice);
stringSet.has(stringSlice);
"#,
    );

    session.assert_dir_checked_diagnostics("main.ds", "");
}
