use crate::tests::{DirRows, TestSession};

#[test]
fn test_record_flow_conclusions_for_a_checked_module() {
    let session = TestSession::single(
        r#"
function run(): int32 {
    let counted = 1;
    counted = 2;
    const idle = 3;
    const observe = () => counted;
    return counted;
    counted;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
function run(): int32 {
    let counted: float64 = 1;
    counted = 2;
    const idle: 3 = 3;
    const observe: () => float64 = (): float64 => counted;
    return counted;
    counted;
}

=== checked ===
function run(): int32 {
    let counted = 1;
    /// @flow.use symbol=counted uses=read+written+captured

    counted = 2;
    const idle = 3;
    const observe = () => counted;
    return counted;
    /// @flow.diverging source="return counted"

    counted;
    /// @flow.unreachable source=counted

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'float64' is not assignable to the declared result type 'int32'"
/// @diagnostic.label line=7 column=12 span="counted" line_source="return counted;"
"#,
    );
}

#[test]
fn test_record_field_and_foreign_uses_in_the_flow_segment() {
    let session = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function max(left: int32, right: int32): int32 {
    return left > right ? left : right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { max } from "./math.ds";

struct Point {
    x: int32;
    y: int32;
}

function shift(point: &Point): int32 {
    point.x = point.x + 1;

    return max(point.x, point.y);
}
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
import { max } from "./math.ds";

struct Point {
    x: int32;
    y: int32;
}

function shift<'a>(point: &'a Point): int32 {
    point.x = point.x + 1;

    return max(point.x, point.y);
}

=== checked ===
import { max } from "./math.ds";

struct Point {
/// @flow.use symbol=Point uses=read

    x: int32;
    /// @flow.use symbol=x uses=read+written

    y: int32;
    /// @flow.use symbol=y uses=read

}

function shift(point: &Point): int32 {
/// @flow.use symbol=point uses=read

    point.x = point.x + 1;

    return max(point.x, point.y);
    /// @flow.diverging source="return max(point.x, point.y)"

}

/// @flow.foreign symbol=math.max uses=read
"#,
        r#"
"#,
    );
}

#[test]
fn test_record_unreachable_branches_in_the_flow_segment() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean): int32 {
    if (flag) {
        return 1;
        flag;
    }

    loop {
        break;
    }

    return 2;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
function pick(flag: boolean): int32 {
    if (flag) {
        return 1;
        flag;
    }

    loop {
        break;
    }

    return 2;
}

=== checked ===
function pick(flag: boolean): int32 {
/// @flow.use symbol=flag uses=read

    if (flag) {
        return 1;
        /// @flow.diverging source="return 1"

        flag;
        /// @flow.unreachable source=flag

    }

    loop {
        break;
        /// @flow.diverging source=break

    }

    return 2;
    /// @flow.diverging source="return 2"

}
"#,
        r#"
"#,
    );
}
