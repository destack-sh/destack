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
        DirRows::imports().with_summaries(),
        r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
/// @reference.target source=x kind=bound targets=[x#1]
/// @reference.target source=label kind=bound targets=[label]
/// @reference.target source=point kind=bound targets=[point]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=6
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
        DirRows::imports().with_summaries(),
        r#"
type User = string;
let value: User;
/// @reference.target source=User kind=bound targets=[User]

/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=1
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
        DirRows::imports().with_summaries(),
        r#"
type Args<T> = T extends (...parameters: infer P) => unknown ? P : never;
/// @reference.target source=T kind=bound targets=[Args.T]
/// @reference.target source=P kind=bound targets=[Args.P]

/// @import.summary
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
        DirRows::imports().with_summaries(),
        r#"
let value: User;
/// @reference.target source=User kind=bound targets=[User]

type User = string;

/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=1
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
        DirRows::imports().with_summaries(),
        r#"
const value = answer;
/// @reference.target source=answer kind=bound targets=[answer]

const answer = 1;

/// @import.summary
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
        DirRows::imports().with_summaries(),
        r#"
const value = missing;
/// @reference.target source=missing kind=missing

/// @import.summary
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
        DirRows::imports().with_summaries(),
        r#"
let value: Missing;
/// @reference.target source=Missing kind=missing

/// @import.summary
/// @reference.summary references=1
"#,
    );
}
