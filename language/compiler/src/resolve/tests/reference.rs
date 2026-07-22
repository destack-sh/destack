use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_assignment_pattern_ignores_structural_member_names() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
/// @reference.bound source=x targets=[x#1]
/// @reference.bound source=label targets=[label]
/// @reference.bound source=point targets=[point]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=6
/// @resolve.stats roots=4 expressions=9 types=5 language=uses:6
/// @reference.summary references=3
"#,
    );
}

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

/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=1
/// @resolve.stats roots=2 expressions=2 types=2 language=uses:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_conditional_infer_branch_reference() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
type Args<T> = T extends (...parameters: infer P) => unknown ? P : never;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
type Args<T> = T extends (...parameters: infer P) => unknown ? P : never;
/// @reference.bound source=T targets=[Args.T]
/// @reference.bound source=P targets=[Args.P]

/// @import.summary
/// @resolve.stats roots=1 expressions=1 types=7
/// @reference.summary references=2
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

/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=1
/// @resolve.stats roots=2 expressions=2 types=2 language=uses:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_binds_local_value_reference_ahead_of_declaration() {
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
/// @reference.bound source=answer targets=[answer]

const answer = 1;

/// @import.summary
/// @resolve.stats roots=2 expressions=4 types=0
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
