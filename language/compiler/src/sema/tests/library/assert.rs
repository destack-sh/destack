use crate::tests::{DirRows, TestSession};

/// Check assertion calls through borrowed operands and intrinsic conformance.
#[test]
fn test_check_assertions() {
    let session = TestSession::single(
        r#"
import { assert, assertEqual, assertNotEqual } from "tspp:assert";

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
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
import { assert, assertEqual, assertNotEqual } from "tspp:assert";

struct Point {
    x: int32;
}

declare const left: Point;
declare const right: Point;

assert(true);
assert(
    true,
    ((): string => "lazy assertion failure") as string | ^Function<(), string, "once"> | undefined,
);
assertEqual<Point, Point, "static", "static">(left as &'static immutable Point, right);
assertEqual<Point, Point, "static", "static">(
    left as &'static immutable Point,
    right,
    ((): string => "lazy equality failure") as string | ^Function<(), string, "once"> | undefined,
);
assertNotEqual<Point, Point, "static", "static">(
    left as &'static immutable Point,
    right,
    "points must differ" as string | ^Function<(), string, "once"> | undefined,
);
assertNotEqual<Point, Point, "static", "static">(
    left as &'static immutable Point,
    right,
    ((): string => "lazy inequality failure") as string | ^Function<(), string, "once"> | undefined,
);
const x: int32 = left.x;

=== dir ===
import { assert, assertEqual, assertNotEqual } from "tspp:assert";

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
        r#"
"#,
    );
}
