use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_local_type_reference() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
type User = string;
let value: User;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
type User = string;
let value: User;
/// @reference.bound source=User targets=[User]

/// @import.summary
/// @resolve.stats roots=2 expressions=2 types=2
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_hoists_local_type_references() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let value: User;
type User = string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let value: User;
/// @reference.bound source=User targets=[User]

type User = string;

/// @import.summary
/// @resolve.stats roots=2 expressions=2 types=2
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_does_not_hoist_local_value_references() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
const value = answer;
const answer = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const value = answer;
/// @reference.missing source=answer

const answer = 1;

/// @import.summary
/// @resolve.stats roots=2 expressions=4 types=0 globals=required:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_unresolved_value_name() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
const value = missing;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const value = missing;
/// @reference.missing source=missing

/// @import.summary
/// @resolve.stats roots=1 expressions=2 types=0 globals=required:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_unresolved_type_name() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let value: Missing;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let value: Missing;
/// @reference.missing source=Missing

/// @import.summary
/// @resolve.stats roots=1 expressions=1 types=1 globals=required:1
/// @reference.summary references=1
"#,
    );
}
