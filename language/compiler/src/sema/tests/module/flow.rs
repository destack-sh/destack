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

    session.assert_dir_and_diagnostics(
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

=== dir ===
function run(): int32 {
    let counted = 1;
    /// @flow.use symbol=counted uses=read+written+captured

    counted = 2;
    /// @flow.access source=counted root=run.counted uses=written

    const idle = 3;
    const observe = () => counted;
    /// @flow.access source=counted root=run.counted uses=read

    return counted;
    /// @flow.diverging source="return counted"
    /// @flow.access source=counted root=run.counted uses=read

    counted;
    /// @flow.unreachable source=counted
    /// @flow.access source=counted root=run.counted uses=read

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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=written
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=read

    return max(point.x, point.y);
    /// @flow.diverging source="return max(point.x, point.y)"
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=read
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.y root=shift.point keys=[y] uses=read

}

/// @flow.foreign symbol=math.max uses=read
"#,
        r#"
"#,
    );
}

#[test]
fn test_record_mutable_binding_storage_uses() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(&exclusive this): void {
        this.value += 1;
    }
}

class Service {
    value: int32 = 0;

    increment(&exclusive this): void {
        this.value += 1;
    }
}

declare function modify(value: &int32): void;

let borrowed: int32 = 0;
&borrowed;

let passed: int32 = 0;
modify(passed);

let field: Counter = Counter { value: 0 };
field.value = 1;

let called: Counter = Counter { value: 0 };
called.increment();

let referenced: Service = new Service();
referenced.increment();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(&exclusive this): void {
        this.value += 1;
    }
}

class Service {
    value: int32 = 0;

    increment(&exclusive this): void {
        this.value += 1;
    }
}

declare function modify<'a>(value: &'a int32): void;

let borrowed: int32 = 0;
&borrowed;

let passed: int32 = 0;
modify(passed as &'static int32);

let field: Counter = Counter { value: 0 };
field.value = 1;

let called: Counter = Counter { value: 0 };
called.increment();

let referenced: Service = new Service();
referenced.increment();

=== dir ===
struct Counter {
/// @flow.use symbol=Counter uses=read

    value: int32;
    /// @flow.use symbol=value#1 uses=read+written

    increment(&exclusive this): void {
    /// @flow.use symbol=increment#1 uses=read

        this.value += 1;
        /// @flow.access source=this.value root=this keys=[value] uses=written+mutable

    }
}

class Service {
/// @flow.use symbol=Service uses=read

    value: int32 = 0;
    /// @flow.use symbol=value#2 uses=read+written

    increment(&exclusive this): void {
    /// @flow.use symbol=increment#2 uses=read

        this.value += 1;
        /// @flow.access source=this.value root=this keys=[value] uses=written+mutable

    }
}

declare function modify(value: &int32): void;
/// @flow.use symbol=modify uses=read

let borrowed: int32 = 0;
/// @flow.use symbol=borrowed uses=read+mutable

&borrowed;
/// @flow.access source=borrowed root=borrowed uses=read+mutable

let passed: int32 = 0;
/// @flow.use symbol=passed uses=read+mutable

modify(passed);
/// @flow.access source=passed root=passed uses=read+mutable

let field: Counter = Counter { value: 0 };
/// @flow.use symbol=field uses=read+mutable

field.value = 1;
/// @flow.access source=field root=field uses=read
/// @flow.access source=field.value root=field keys=[value] uses=written+mutable

let called: Counter = Counter { value: 0 };
/// @flow.use symbol=called uses=read+mutable

called.increment();
/// @flow.access source=called root=called uses=read+mutable

let referenced: Service = new Service();
/// @flow.use symbol=referenced uses=read

referenced.increment();
/// @flow.access source=referenced root=referenced uses=read
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
function pick(flag: boolean): int32 {
/// @flow.use symbol=flag uses=read

    if (flag) {
    /// @flow.access source=flag root=pick.flag uses=read

        return 1;
        /// @flow.diverging source="return 1"

        flag;
        /// @flow.unreachable source=flag
        /// @flow.access source=flag root=pick.flag uses=read

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
