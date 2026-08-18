use crate::tests::TestSession;

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
declare const stringValue: string;
declare const bigintValue: bigint;

requireEqual(booleanValue);
requireEqual(characterValue);
requireEqual(integerValue);
requirePartialEqual(floatValue);
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

    session.assert_dir_diagnostics("main.ds", "");
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

    session.assert_dir_diagnostics(
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

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'PartialEqual<string>'"
/// @diagnostic.label line=7 column=1 span="requireStringEqual(value)" line_source="requireStringEqual(value);"
/// @diagnostic.related line=4 column=37 span="T" line_source="declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}
