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
assert(true, () => "lazy assertion failure");
assertEqual(left, right);
assertEqual(left, right, () => "lazy equality failure");
assertNotEqual(left, right, "points must differ");
assertNotEqual(left, right, () => "lazy inequality failure");
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
assert(true, ((): string => "lazy assertion failure") as AssertionMessage | undefined);
assertEqual<Point, Point, "constant", "constant">(
    left as &'static readonly Point,
    right as &'static readonly Point,
);
assertEqual<Point, Point, "constant", "constant">(
    left as &'static readonly Point,
    right as &'static readonly Point,
    ((): string => "lazy equality failure") as AssertionMessage | undefined,
);
assertNotEqual<Point, Point, "constant", "constant">(
    left as &'static readonly Point,
    right as &'static readonly Point,
    "points must differ" as AssertionMessage | undefined,
);
assertNotEqual<Point, Point, "constant", "constant">(
    left as &'static readonly Point,
    right as &'static readonly Point,
    ((): string => "lazy inequality failure") as AssertionMessage | undefined,
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
assert(true, () => "lazy assertion failure");
assertEqual(left, right);
assertEqual(left, right, () => "lazy equality failure");
assertNotEqual(left, right, "points must differ");
assertNotEqual(left, right, () => "lazy inequality failure");
const x = left.x;
"#,
        "",
    );
}
