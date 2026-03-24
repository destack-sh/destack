use super::*;
use crate::TaskPhase;
use destack_artifact::ArtifactKey;
use std::time::Duration;

/// Check that malformed syntax stays parse-level damage.
fn assert_stays_parse_local(source: &str, file_name: &str) {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(file_name, source);

    // run the early pipeline with a tight timeout so recovery regressions fail fast
    test.analyze_module(module_id);
    test.compile_with_timeout(Duration::from_secs(2));

    // malformed syntax should not leak into late compiler phases
    test.check_no_diagnostics_for_phases(&[TaskPhase::Resolve, TaskPhase::Analyze]);
}

/// Check that malformed syntax does not wedge one specific artifact stage.
fn assert_stage_completes(
    source: &str,
    file_name: &str,
    enqueue: impl FnOnce(&TestProgram, ModuleId),
) {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(file_name, source);

    enqueue(&test, module_id);
    test.compile_with_timeout(Duration::from_secs(2));
}

#[test]
fn test_recover_malformed_declare_module_body_stays_parse_local() {
    assert_stays_parse_local(
        r##"
declare module A {
    "name": "troublesome-lib",
    "typings": "lib/index.d.ts",
    "version": "0.0.1"
}
"##,
        "test.d.ts",
    );
}

#[test]
fn test_recover_malformed_call_argument_lists_stay_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b;
"##,
        "missing-close.js",
    );

    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
"##,
        "missing-close-before-var.js",
    );

    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo (,,b);
"##,
        "leading-empty-slots.js",
    );

    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo (a, ...);
"##,
        "trailing-spread.js",
    );
}

#[test]
fn test_recover_combined_malformed_call_argument_lists_stay_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b;
foo(a,b var;
foo (,,b);
foo (a, ...);
"##,
        "combined-invalid-arg-list.js",
    );
}

#[test]
fn test_recover_malformed_call_then_var_statement_stays_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b;
foo(a,b var;
"##,
        "call-then-var.js",
    );
}

#[test]
fn test_recover_var_statement_then_empty_slots_call_stays_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (,,b);
"##,
        "var-then-empty-slots.js",
    );
}

#[test]
fn test_recover_var_statement_then_trailing_spread_call_stays_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (a, ...);
"##,
        "var-then-trailing-spread.js",
    );
}

#[test]
fn test_recover_var_statement_then_two_following_malformed_calls_stays_parse_local() {
    assert_stays_parse_local(
        r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (,,b);
foo (a, ...);
"##,
        "var-then-two-following-calls.js",
    );
}

#[test]
fn test_recover_var_statement_then_two_following_malformed_calls_resolves() {
    let source = r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (,,b);
foo (a, ...);
"##;

    assert_stage_completes(
        source,
        "var-then-two-following-calls-resolve.js",
        |test, module_id| {
            test.resolve_module(module_id);
        },
    );
}

#[test]
fn test_recover_var_statement_then_two_following_malformed_calls_declares() {
    let source = r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (,,b);
foo (a, ...);
"##;

    assert_stage_completes(
        source,
        "var-then-two-following-calls-declare.js",
        |test, module_id| {
            let profile = test.default_profile_id(module_id);
            test.enqueue(ArtifactKey::dir_declared(module_id, profile));
        },
    );
}

#[test]
fn test_recover_var_statement_then_two_following_malformed_calls_builds_interface() {
    let source = r##"
function foo(...args) {}
let a, b, c;
foo(a,b var;
foo (,,b);
foo (a, ...);
"##;

    assert_stage_completes(
        source,
        "var-then-two-following-calls-interface.js",
        |test, module_id| {
            let profile = test.default_profile_id(module_id);
            test.enqueue(ArtifactKey::dir_interface(module_id, profile));
        },
    );
}
