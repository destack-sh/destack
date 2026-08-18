use crate::tests::{DirRows, TestSession};

/// Check assertion calls through borrowed operands and intrinsic conformance.
#[test]
fn test_check_assertions() {
    let session = TestSession::single(
        r#"
import { assert, assertEqual, assertNotEqual } from "destack:assert";

struct Point {
    x: int32;
}

declare const left: Point;
declare const right: Point;

assert(true);
assertEqual(left, right);
assertNotEqual(left, right, "points must differ");
const x = left.x;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { assert, assertEqual, assertNotEqual } from "destack:assert";

struct Point {
    x: int32;
}

declare const left: Point;
declare const right: Point;

assert(true);
assertEqual<Point, Point>(left as &'static readonly Point, right as &'static readonly Point);
assertNotEqual<Point, Point>(
    left as &'static readonly Point,
    right as &'static readonly Point,
    "points must differ" as string | undefined,
);
const x: int32 = left.x;

=== dir ===
import { assert, assertEqual, assertNotEqual } from "destack:assert";

struct Point {
    x: int32;
}

declare const left: Point;
declare const right: Point;

assert(true);
assertEqual(left, right);
assertNotEqual(left, right, "points must differ");
const x = left.x;
"#,
        "",
    );
}
